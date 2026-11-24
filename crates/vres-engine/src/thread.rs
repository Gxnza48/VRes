//! El hilo del motor: una ventana oculta y su bucle de mensajes.
//!
//! Solo este hilo toca la Magnification API, registra el atajo de emergencia
//! y recibe los avisos del sistema. Los demás hilos le hablan por un canal
//! ([`EngineHandle::send`]) y él les responde con eventos ([`EngineEvent`]).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use vres_core::color::matrix::ColorMatrix;
use vres_core::color::ramp::GammaRamp;
use vres_core::model::display::DisplayInfo;
use vres_core::planner::Requested;
use vres_core::status::{EngineStatus, ReasonCode};
use vres_win::control::{self, ACK, RemoteCommand};
use vres_win::identify::IdentifyOverlay;
use vres_win::mag::Magnifier;
use vres_win::window::{self, ForegroundHook, HiddenWindow};
use vres_win::{displays, gamma};

use crate::core::{Core, Hardware};

// Mensajes y constantes públicas de la API de Windows.
const WM_CLOSE: u32 = 0x0010;
const WM_QUERYENDSESSION: u32 = 0x0011;
const WM_ENDSESSION: u32 = 0x0016;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_TIMER: u32 = 0x0113;
const WM_POWERBROADCAST: u32 = 0x0218;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WM_HOTKEY: u32 = 0x0312;
const WM_APP: u32 = 0x8000;
const WTS_SESSION_UNLOCK: usize = 0x8;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
const MOD_ALT: u32 = 0x1;
const MOD_CONTROL: u32 = 0x2;
const MOD_SHIFT: u32 = 0x4;
const MOD_NOREPEAT: u32 = 0x4000;
const VK_R: u32 = 0x52;

/// Hay comandos en el canal.
const WM_WAKE: u32 = WM_APP + 1;
/// Cambió la ventana en primer plano.
const WM_FOREGROUND: u32 = WM_APP + 2;

const TIMER_SETTLE: usize = 1;
const TIMER_HEALTH: usize = 2;
const TIMER_IDENTIFY: usize = 3;
const TIMER_FOREGROUND: usize = 4;
const HOTKEY_EMERGENCY: i32 = 1;

const HEALTH_INTERVAL_MS: u32 = 1000;
const IDENTIFY_MS: u32 = 2500;

/// Configuración del motor.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// `restore.json`. `None` desactiva la recuperación tras un cierre inesperado.
    pub restore_path: Option<PathBuf>,
    /// Lo que se aplica al arrancar.
    pub requested: Requested,
}

/// Vista previa (mientras se arrastra un control) o cambio confirmado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    Preview,
    Commit,
}

/// Pedidos al motor.
#[derive(Debug)]
pub enum Command {
    /// Aplicar lo pedido. Si llegan varios juntos, solo cuenta el último.
    Set(Box<Requested>, ApplyMode),
    /// Mostrar el número de cada pantalla.
    Identify,
    /// Volver todo a la normalidad y apagar el color.
    EmergencyReset,
    /// Volver a leer las pantallas y reaplicar.
    Refresh,
    /// Restaurar todo y terminar el hilo.
    Shutdown(Sender<()>),
}

/// Avisos del motor.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Cambió el estado de alguna capa.
    Status(EngineStatus),
    /// Cambiaron las pantallas.
    Displays(Vec<DisplayInfo>),
    /// Se restableció todo (atajo de emergencia u otro proceso): el color quedó apagado.
    EmergencyReset,
    /// Otro proceso pidió cerrar VRes (`vres.exe --shutdown`, el instalador).
    QuitRequested,
    /// Aviso para el usuario.
    Notice(ReasonCode),
}

/// Estado inicial, al terminar de arrancar.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub displays: Vec<DisplayInfo>,
    pub status: EngineStatus,
    /// Se restauraron cambios de una sesión que no cerró bien.
    pub recovered: bool,
    /// Se pudo registrar el atajo de emergencia (`Ctrl + Alt + Shift + R`).
    pub hotkey_registered: bool,
}

/// Carpeta de estado de la máquina: `%LOCALAPPDATA%\io.github.gxnza48.vres\state`.
pub fn state_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| {
        PathBuf::from(dir)
            .join(vres_core::APP_IDENTIFIER)
            .join("state")
    })
}

/// `restore.json`: los valores originales mientras VRes tiene cambios aplicados.
pub fn restore_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("restore.json"))
}

/// Si quedaron cambios de una sesión que no cerró bien, los deshace sin
/// arrancar la app (lo usan `vres.exe --reset` y `vres-cli reset`).
/// Devuelve `true` si había algo que restaurar.
pub fn recover_from_disk() -> bool {
    let Some(path) = restore_path() else {
        return false;
    };
    let found = displays::enumerate().unwrap_or_default();
    let mut core = Core::new(WinHardware { magnifier: None }, Some(path));
    core.recover(&found)
}

/// Función que recibe los eventos. Corre en el hilo del motor: no debe bloquear.
pub type EventSink = Arc<dyn Fn(EngineEvent) + Send + Sync>;

/// Manija del motor, para hablarle desde cualquier hilo.
#[derive(Debug)]
pub struct EngineHandle {
    tx: Sender<Command>,
    hwnd: isize,
    wake_pending: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Envía un pedido. Devuelve `false` si el motor ya terminó.
    pub fn send(&self, command: Command) -> bool {
        if self.tx.send(command).is_err() {
            return false;
        }
        // Como mucho un aviso pendiente: el motor vacía el canal entero.
        if !self.wake_pending.swap(true, Ordering::AcqRel) {
            window::post_message(self.hwnd, WM_WAKE, 0, 0);
        }
        true
    }

    /// Restaura todo y espera a que el motor termine (con tope `timeout`).
    pub fn shutdown(&mut self, timeout: Duration) -> bool {
        let (reply_tx, reply_rx) = mpsc::channel();
        if !self.send(Command::Shutdown(reply_tx)) {
            return false;
        }
        let done = reply_rx.recv_timeout(timeout).is_ok();
        if done && let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        done
    }
}

/// Rampas originales de las pantallas modificadas, para restaurar desde
/// cualquier hilo (por ejemplo, el hook de pánico).
static EMERGENCY_ORIGINALS: Mutex<Vec<(String, GammaRamp)>> = Mutex::new(Vec::new());

/// Restauración de emergencia. Se puede llamar desde cualquier hilo. La matriz
/// de color no hace falta: Windows la quita sola al terminar el proceso.
pub fn emergency_restore() {
    for _ in 0..20 {
        if let Ok(originals) = EMERGENCY_ORIGINALS.try_lock() {
            for (gdi_name, ramp) in originals.iter() {
                let _ = gamma::write_ramp(gdi_name, ramp);
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct WinHardware {
    magnifier: Option<Magnifier>,
}

impl Hardware for WinHardware {
    fn set_matrix(&mut self, matrix: &ColorMatrix) -> Result<(), String> {
        let magnifier = self
            .magnifier
            .as_ref()
            .ok_or_else(|| "magnification api unavailable".to_owned())?;
        magnifier
            .set_color_effect(matrix)
            .map_err(|error| error.to_string())
    }

    fn matrix(&mut self) -> Result<ColorMatrix, String> {
        let magnifier = self
            .magnifier
            .as_ref()
            .ok_or_else(|| "magnification api unavailable".to_owned())?;
        magnifier.color_effect().map_err(|error| error.to_string())
    }

    fn read_ramp(&mut self, gdi_name: &str) -> Result<GammaRamp, String> {
        gamma::read_ramp(gdi_name).map_err(|error| error.to_string())
    }

    fn write_ramp(&mut self, gdi_name: &str, ramp: &GammaRamp) -> Result<bool, String> {
        gamma::write_ramp(gdi_name, ramp).map_err(|error| error.to_string())
    }
}

struct Shell {
    core: Core<WinHardware>,
    sink: EventSink,
    rx: Receiver<Command>,
    wake_pending: Arc<AtomicBool>,
    remote_message: u32,
    hwnd: isize,
    identify: Option<IdentifyOverlay>,
    displays_dirty: bool,
    health_armed: bool,
    shutdown_reply: Option<Sender<()>>,
}

impl Shell {
    fn on_message(&mut self, msg: u32, wparam: usize) -> Option<isize> {
        match msg {
            WM_WAKE => {
                self.drain();
                Some(0)
            }
            WM_HOTKEY => {
                if wparam == HOTKEY_EMERGENCY as usize {
                    self.emergency_reset();
                }
                Some(0)
            }
            WM_TIMER => {
                self.on_timer(wparam);
                Some(0)
            }
            WM_DISPLAYCHANGE => {
                self.displays_dirty = true;
                window::set_timer(self.hwnd, TIMER_SETTLE, 500);
                None
            }
            WM_WTSSESSION_CHANGE => {
                if wparam == WTS_SESSION_UNLOCK {
                    window::set_timer(self.hwnd, TIMER_SETTLE, 300);
                }
                None
            }
            WM_POWERBROADCAST => {
                if wparam == PBT_APMRESUMEAUTOMATIC {
                    self.displays_dirty = true;
                    window::set_timer(self.hwnd, TIMER_SETTLE, 1000);
                }
                None
            }
            WM_FOREGROUND => {
                // Un juego a pantalla completa puede pisar la rampa: se verifica.
                if self.core.is_active() {
                    window::set_timer(self.hwnd, TIMER_FOREGROUND, 150);
                }
                Some(0)
            }
            WM_QUERYENDSESSION => Some(1),
            WM_ENDSESSION => {
                if wparam != 0 {
                    self.core.restore_all();
                    self.after_change(true);
                }
                Some(0)
            }
            message if message == self.remote_message => Some(self.on_remote(wparam)),
            _ => None,
        }
    }

    fn on_remote(&mut self, wparam: usize) -> isize {
        match RemoteCommand::from_wparam(wparam) {
            Some(RemoteCommand::EmergencyReset) => self.emergency_reset(),
            // La app decide cómo salir; el motor restaura en `Shutdown`.
            Some(RemoteCommand::Shutdown) => (self.sink)(EngineEvent::QuitRequested),
            Some(RemoteCommand::Ping) => {}
            None => return 0,
        }
        ACK
    }

    fn on_timer(&mut self, id: usize) {
        match id {
            TIMER_SETTLE => {
                window::kill_timer(self.hwnd, TIMER_SETTLE);
                if std::mem::take(&mut self.displays_dirty) {
                    self.refresh_displays();
                }
                // Windows resetea las rampas en los cambios de pantalla: se reescribe todo.
                let changed = self.core.reconcile(true);
                self.after_change(changed);
            }
            TIMER_HEALTH => {
                let changed = self.core.health_check();
                self.after_change(changed);
            }
            TIMER_FOREGROUND => {
                window::kill_timer(self.hwnd, TIMER_FOREGROUND);
                let changed = self.core.health_check();
                self.after_change(changed);
            }
            TIMER_IDENTIFY => {
                window::kill_timer(self.hwnd, TIMER_IDENTIFY);
                self.identify = None;
            }
            _ => {}
        }
    }

    fn drain(&mut self) {
        self.wake_pending.store(false, Ordering::Release);
        let mut latest: Option<Requested> = None;
        let (mut identify, mut emergency, mut refresh) = (false, false, false);
        let mut shutdown = None;
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Set(requested, _) => latest = Some(*requested),
                Command::Identify => identify = true,
                Command::EmergencyReset => emergency = true,
                Command::Refresh => refresh = true,
                Command::Shutdown(reply) => shutdown = Some(reply),
            }
        }

        if let Some(requested) = latest {
            self.core.set_requested(requested);
            let changed = self.core.reconcile(false);
            self.after_change(changed);
        }
        if refresh {
            self.displays_dirty = true;
            window::set_timer(self.hwnd, TIMER_SETTLE, 10);
        }
        if identify {
            self.identify();
        }
        if emergency {
            self.emergency_reset();
        }
        if let Some(reply) = shutdown {
            self.core.restore_all();
            self.after_change(true);
            self.identify = None;
            self.shutdown_reply = Some(reply);
            window::post_message(self.hwnd, WM_CLOSE, 0, 0);
        }
    }

    fn refresh_displays(&mut self) {
        let Ok(found) = displays::enumerate() else {
            return;
        };
        if found.as_slice() != self.core.displays() {
            self.core.set_displays(found.clone());
            (self.sink)(EngineEvent::Displays(found));
        }
    }

    fn emergency_reset(&mut self) {
        let mut requested = self.core.requested().clone();
        requested.color_enabled = false;
        self.core.set_requested(requested);
        self.core.restore_all();
        self.after_change(true);
        (self.sink)(EngineEvent::EmergencyReset);
    }

    fn identify(&mut self) {
        let entries: Vec<_> = self
            .core
            .displays()
            .iter()
            .enumerate()
            .map(|(index, display)| (display.bounds, index as u32 + 1))
            .collect();
        self.identify = None;
        if let Ok(overlay) = IdentifyOverlay::show(&entries) {
            self.identify = Some(overlay);
            window::set_timer(self.hwnd, TIMER_IDENTIFY, IDENTIFY_MS);
        }
    }

    fn after_change(&mut self, status_changed: bool) {
        if status_changed {
            (self.sink)(EngineEvent::Status(self.core.status().clone()));
        }
        if let Ok(mut originals) = EMERGENCY_ORIGINALS.lock() {
            *originals = self.core.originals_to_restore();
        }
        let active = self.core.is_active();
        if active && !self.health_armed {
            window::set_timer(self.hwnd, TIMER_HEALTH, HEALTH_INTERVAL_MS);
        } else if !active && self.health_armed {
            window::kill_timer(self.hwnd, TIMER_HEALTH);
        }
        self.health_armed = active;
    }
}

/// Arranca el motor en su propio hilo. Devuelve la manija y el estado inicial.
pub fn spawn(
    config: EngineConfig,
    sink: EventSink,
) -> Result<(EngineHandle, EngineSnapshot), String> {
    let (tx, rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(isize, EngineSnapshot), String>>();
    let wake_pending = Arc::new(AtomicBool::new(false));
    let thread_wake = Arc::clone(&wake_pending);

    let thread = std::thread::Builder::new()
        .name("vres-engine".into())
        .spawn(move || run(config, sink, rx, thread_wake, ready_tx))
        .map_err(|error| error.to_string())?;

    match ready_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok((hwnd, snapshot))) => Ok((
            EngineHandle {
                tx,
                hwnd,
                wake_pending,
                thread: Some(thread),
            },
            snapshot,
        )),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("engine did not start in time".into()),
    }
}

type Ready = Sender<Result<(isize, EngineSnapshot), String>>;

fn run(
    config: EngineConfig,
    sink: EventSink,
    rx: Receiver<Command>,
    wake_pending: Arc<AtomicBool>,
    ready: Ready,
) {
    // La sesión de Magnification es de este hilo.
    let magnifier = Magnifier::init().ok();
    let mut core = Core::new(WinHardware { magnifier }, config.restore_path);

    // Primero se deshace lo que haya quedado de una sesión anterior; recién
    // después se capturan los originales.
    let found = displays::enumerate().unwrap_or_default();
    let recovered = core.recover(&found);
    core.set_displays(found);
    core.set_requested(config.requested);
    core.reconcile(false);

    let remote_message = window::register_message(control::REMOTE_MESSAGE_NAME);
    let shell = Rc::new(RefCell::new(Shell {
        core,
        sink,
        rx,
        wake_pending,
        remote_message,
        hwnd: 0,
        identify: None,
        displays_dirty: false,
        health_armed: false,
        shutdown_reply: None,
    }));

    let handler_shell = Rc::clone(&shell);
    let window = match HiddenWindow::create(
        control::WINDOW_CLASS_NAME,
        Box::new(move |_hwnd, msg, wparam, _lparam| {
            handler_shell
                .try_borrow_mut()
                .ok()
                .and_then(|mut shell| shell.on_message(msg, wparam))
        }),
    ) {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    window.allow_from_lower_integrity(remote_message);
    let hotkey_ok = window
        .register_hotkey(
            HOTKEY_EMERGENCY,
            MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT,
            VK_R,
        )
        .is_ok();
    window.register_session_notifications();
    let foreground = ForegroundHook::install(window.handle(), WM_FOREGROUND).ok();

    if let Ok(mut shell) = shell.try_borrow_mut() {
        shell.hwnd = window.handle();
        shell.after_change(false);
        let snapshot = EngineSnapshot {
            displays: shell.core.displays().to_vec(),
            status: shell.core.status().clone(),
            recovered,
            hotkey_registered: hotkey_ok,
        };
        let _ = ready.send(Ok((window.handle(), snapshot)));
    }

    window::run_message_loop();

    drop(foreground);
    window.unregister_hotkey(HOTKEY_EMERGENCY);
    drop(window);
    if let Ok(mut shell) = shell.try_borrow_mut()
        && let Some(reply) = shell.shutdown_reply.take()
    {
        let _ = reply.send(());
    }
}
