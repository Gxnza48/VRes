//! Textos nativos (bandeja), que tienen que existir aunque la interfaz no esté
//! abierta. El resto de los textos vive en `src/locales`.

/// Idiomas soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
    PtBr,
}

impl Lang {
    /// Elige el idioma a partir de una etiqueta BCP 47 (`es-AR`, `pt-BR`…).
    /// Español es el idioma por defecto.
    pub fn from_tag(tag: Option<&str>) -> Self {
        let tag = tag.unwrap_or_default().to_ascii_lowercase();
        if tag.starts_with("pt") {
            Self::PtBr
        } else if tag.starts_with("en") {
            Self::En
        } else {
            Self::Es
        }
    }

    /// Idioma del sistema.
    pub fn system() -> Self {
        Self::from_tag(sys_locale::get_locale().as_deref())
    }
}

/// Textos del menú de la bandeja.
#[derive(Debug, Clone, Copy)]
pub struct TrayTexts {
    pub tooltip: &'static str,
    pub open: &'static str,
    pub quick: &'static str,
    pub pause: &'static str,
    pub reset: &'static str,
    pub quit: &'static str,
}

/// Textos de la bandeja en el idioma pedido.
pub fn tray_texts(lang: Lang) -> TrayTexts {
    match lang {
        Lang::Es => TrayTexts {
            tooltip: "VRes",
            open: "Abrir VRes",
            quick: "Ajuste rápido",
            pause: "Pausar todo",
            reset: "Restablecer pantalla",
            quit: "Salir",
        },
        Lang::En => TrayTexts {
            tooltip: "VRes",
            open: "Open VRes",
            quick: "Quick adjust",
            pause: "Pause everything",
            reset: "Reset display",
            quit: "Quit",
        },
        Lang::PtBr => TrayTexts {
            tooltip: "VRes",
            open: "Abrir o VRes",
            quick: "Ajuste rápido",
            pause: "Pausar tudo",
            reset: "Restaurar tela",
            quit: "Sair",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_language_from_tag() {
        assert_eq!(Lang::from_tag(Some("es-AR")), Lang::Es);
        assert_eq!(Lang::from_tag(Some("pt-BR")), Lang::PtBr);
        assert_eq!(Lang::from_tag(Some("en-US")), Lang::En);
        assert_eq!(Lang::from_tag(Some("fr-FR")), Lang::Es);
        assert_eq!(Lang::from_tag(None), Lang::Es);
    }
}
