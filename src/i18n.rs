//! Centralized user-interface translations.
//!
//! Keep human-facing strings here instead of scattering literals throughout
//! the wxDragon front end.  Calculator operator/function labels are deliberately
//! kept language-neutral where Windows Calculator traditionally used compact
//! mathematical abbreviations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Portuguese,
    Spanish,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

impl Language {
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Portuguese => "pt",
            Self::Spanish => "es",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" | "en-us" | "en_us" => Some(Self::English),
            "pt" | "por" | "portuguese" | "português" | "pt-br" | "pt_br" => {
                Some(Self::Portuguese)
            }
            "es" | "spa" | "spanish" | "español" | "es-es" | "es_es" => {
                Some(Self::Spanish)
            }
            _ => None,
        }
    }

    /// Language names are intentionally autonyms so the Language submenu stays
    /// understandable even after switching to an unfamiliar language.
    pub const fn autonym(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
            Self::Spanish => "Español",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Strings {
    language: Language,
}

impl Strings {
    pub const fn new(language: Language) -> Self {
        Self { language }
    }

    pub const fn calculator_title(self) -> &'static str {
        match self.language {
            Language::English => "Calculator",
            Language::Portuguese => "Calculadora",
            Language::Spanish => "Calculadora",
        }
    }

    pub const fn edit_menu(self) -> &'static str {
        match self.language {
            Language::English => "&Edit",
            Language::Portuguese => "&Editar",
            Language::Spanish => "&Editar",
        }
    }


    pub const fn undo(self) -> &'static str {
        match self.language {
            Language::English => "&Undo\tCtrl+Z",
            Language::Portuguese => "&Desfazer\tCtrl+Z",
            Language::Spanish => "&Deshacer\tCtrl+Z",
        }
    }

    pub const fn redo(self) -> &'static str {
        match self.language {
            Language::English => "&Redo\tCtrl+Y",
            Language::Portuguese => "&Refazer\tCtrl+Y",
            Language::Spanish => "&Rehacer\tCtrl+Y",
        }
    }

    pub const fn copy(self) -> &'static str {
        match self.language {
            Language::English => "&Copy\tCtrl+C",
            Language::Portuguese => "&Copiar\tCtrl+C",
            Language::Spanish => "&Copiar\tCtrl+C",
        }
    }

    pub const fn paste(self) -> &'static str {
        match self.language {
            Language::English => "&Paste\tCtrl+V",
            Language::Portuguese => "C&olar\tCtrl+V",
            Language::Spanish => "&Pegar\tCtrl+V",
        }
    }

    pub const fn view_menu(self) -> &'static str {
        match self.language {
            Language::English => "&View",
            Language::Portuguese => "E&xibir",
            Language::Spanish => "&Ver",
        }
    }

    pub const fn scientific(self) -> &'static str {
        match self.language {
            Language::English => "&Scientific",
            Language::Portuguese => "&Científica",
            Language::Spanish => "&Científica",
        }
    }

    pub const fn standard(self) -> &'static str {
        match self.language {
            Language::English => "S&tandard",
            Language::Portuguese => "&Padrão",
            Language::Spanish => "&Estándar",
        }
    }

    pub const fn history(self) -> &'static str {
        match self.language {
            Language::English => "&History",
            Language::Portuguese => "&Histórico",
            Language::Spanish => "&Historial",
        }
    }

    pub const fn history_title(self) -> &'static str {
        match self.language {
            Language::English => "History",
            Language::Portuguese => "Histórico",
            Language::Spanish => "Historial",
        }
    }

    pub const fn clear_history(self) -> &'static str {
        match self.language {
            Language::English => "Clear",
            Language::Portuguese => "Limpar",
            Language::Spanish => "Borrar",
        }
    }

    pub const fn language_menu(self) -> &'static str {
        match self.language {
            Language::English => "&Language",
            Language::Portuguese => "&Idioma",
            Language::Spanish => "&Idioma",
        }
    }

    pub const fn decimal_separator_menu(self) -> &'static str {
        match self.language {
            Language::English => "&Decimal separator",
            Language::Portuguese => "&Separador decimal",
            Language::Spanish => "&Separador decimal",
        }
    }

    pub const fn period_separator(self) -> &'static str {
        match self.language {
            Language::English => "&Period (.)",
            Language::Portuguese => "&Ponto (.)",
            Language::Spanish => "&Punto (.)",
        }
    }

    pub const fn comma_separator(self) -> &'static str {
        match self.language {
            Language::English => "&Comma (,)",
            Language::Portuguese => "&Vírgula (,)",
            Language::Spanish => "&Coma (,)",
        }
    }

    pub const fn graph(self) -> &'static str {
        match self.language {
            Language::English => "&Graph",
            Language::Portuguese => "&Gráfico",
            Language::Spanish => "&Gráfico",
        }
    }

    pub const fn graph_help(self) -> &'static str {
        match self.language {
            Language::English => "Show or hide the graph panel.",
            Language::Portuguese => "Mostrar ou ocultar o painel de gráfico.",
            Language::Spanish => "Mostrar u ocultar el panel de gráfico.",
        }
    }

    pub const fn graph_function(self) -> &'static str {
        match self.language {
            Language::English => "Function:",
            Language::Portuguese => "Função:",
            Language::Spanish => "Función:",
        }
    }

    pub const fn graph_plot(self) -> &'static str {
        match self.language {
            Language::English => "Plot",
            Language::Portuguese => "Traçar",
            Language::Spanish => "Trazar",
        }
    }

    pub const fn graph_reset_view(self) -> &'static str {
        match self.language {
            Language::English => "Reset view",
            Language::Portuguese => "Redefinir vista",
            Language::Spanish => "Restablecer vista",
        }
    }

    pub const fn graph_export(self) -> &'static str {
        match self.language {
            Language::English => "Export",
            Language::Portuguese => "Exportar",
            Language::Spanish => "Exportar",
        }
    }

    pub const fn graph_roots_not_plotted(self) -> &'static str {
        match self.language {
            Language::English => "Roots: plot a function first.",
            Language::Portuguese => "Raízes: trace uma função primeiro.",
            Language::Spanish => "Raíces: trace una función primero.",
        }
    }

    pub const fn graph_roots_visible(self) -> &'static str {
        match self.language {
            Language::English => "Roots in visible range: x = ",
            Language::Portuguese => "Raízes no intervalo visível: x = ",
            Language::Spanish => "Raíces en el intervalo visible: x = ",
        }
    }

    pub const fn graph_no_roots(self) -> &'static str {
        match self.language {
            Language::English => "No roots in visible range.",
            Language::Portuguese => "Nenhuma raiz no intervalo visível.",
            Language::Spanish => "No hay raíces en el intervalo visible.",
        }
    }

    pub const fn graph_infinite_roots(self) -> &'static str {
        match self.language {
            Language::English => "Infinitely many roots in the visible range.",
            Language::Portuguese => "Há infinitas raízes no intervalo visível.",
            Language::Spanish => "Hay infinitas raíces en el intervalo visible.",
        }
    }

    pub const fn graph_roots_unreliable(self) -> &'static str {
        match self.language {
            Language::English => "Unable to determine roots reliably.",
            Language::Portuguese => "Não foi possível determinar as raízes com segurança.",
            Language::Spanish => "No fue posible determinar las raíces de forma fiable.",
        }
    }

    pub const fn graph_export_title(self) -> &'static str {
        match self.language {
            Language::English => "Export graph",
            Language::Portuguese => "Exportar gráfico",
            Language::Spanish => "Exportar gráfico",
        }
    }

    pub const fn graph_export_error(self) -> &'static str {
        match self.language {
            Language::English => "Could not export graph",
            Language::Portuguese => "Não foi possível exportar o gráfico",
            Language::Spanish => "No se pudo exportar el gráfico",
        }
    }

    pub const fn graph_plot_error(self) -> &'static str {
        match self.language {
            Language::English => "Could not plot function",
            Language::Portuguese => "Não foi possível traçar a função",
            Language::Spanish => "No se pudo trazar la función",
        }
    }

    pub const fn help_menu(self) -> &'static str {
        match self.language {
            Language::English => "&Help",
            Language::Portuguese => "&Ajuda",
            Language::Spanish => "A&yuda",
        }
    }

    pub const fn help_topics(self) -> &'static str {
        match self.language {
            Language::English => "&Help Topics\tF1",
            Language::Portuguese => "&Tópicos da Ajuda\tF1",
            Language::Spanish => "&Temas de ayuda\tF1",
        }
    }

    pub const fn about_opencalc(self) -> &'static str {
        match self.language {
            Language::English => "&About OpenCalc",
            Language::Portuguese => "&Sobre o OpenCalc",
            Language::Spanish => "&Acerca de OpenCalc",
        }
    }

    pub const fn about_title(self) -> &'static str {
        match self.language {
            Language::English => "About OpenCalc",
            Language::Portuguese => "Sobre o OpenCalc",
            Language::Spanish => "Acerca de OpenCalc",
        }
    }

    pub const fn about_body(self) -> &'static str {
        match self.language {
            Language::English => "OpenCalc\n\nWindows 95 Calculator reimplementation in Rust\nNative wxDragon interface; corrected expression parser.",
            Language::Portuguese => "OpenCalc\n\nReimplementação da Calculadora do Windows 95 em Rust\nInterface nativa wxDragon; analisador de expressões corrigido.",
            Language::Spanish => "OpenCalc\n\nReimplementación de la Calculadora de Windows 95 en Rust\nInterfaz nativa wxDragon; analizador de expresiones corregido.",
        }
    }

    pub const fn help_title(self) -> &'static str {
        match self.language {
            Language::English => "Calculator Help",
            Language::Portuguese => "Ajuda da Calculadora",
            Language::Spanish => "Ayuda de la Calculadora",
        }
    }

    pub const fn statistics_box_title(self) -> &'static str {
        match self.language {
            Language::English => "Statistics Box",
            Language::Portuguese => "Caixa de Estatística",
            Language::Spanish => "Cuadro de estadísticas",
        }
    }

    pub const fn whats_this(self) -> &'static str {
        match self.language {
            Language::English => "What's This?",
            Language::Portuguese => "O que é isto?",
            Language::Spanish => "¿Qué es esto?",
        }
    }

    pub const fn settings_error_prefix(self) -> &'static str {
        match self.language {
            Language::English => "Could not save calculator settings",
            Language::Portuguese => "Não foi possível salvar as configurações da calculadora",
            Language::Spanish => "No se pudo guardar la configuración de la calculadora",
        }
    }

    /// Translate model/parser/platform messages without coupling those layers to
    /// the UI language.  They keep stable English diagnostic strings internally;
    /// the presentation layer maps them here.
    pub fn runtime_message(self, message: &str) -> Option<&'static str> {
        match self.language {
            Language::English => match message {
                "Cannot divide by zero." => Some("Cannot divide by zero."),
                "Invalid input for function." => Some("Invalid input for function."),
                "Result of function is undefined." => Some("Result of function is undefined."),
                "Result is too large." => Some("Result is too large."),
                "Result is too small." => Some("Result is too small."),
                "Missing digits after numeric base prefix." => Some("Missing digits after numeric base prefix."),
                "Invalid based integer" => Some("Invalid based integer"),
                "Missing identifier" => Some("Missing identifier"),
                "Invalid number." => Some("Invalid number."),
                "Invalid number" => Some("Invalid number"),
                "Missing closing parenthesis." => Some("Missing closing parenthesis."),
                "Unknown operator." => Some("Unknown operator."),
                "Calculator is in error state." => Some("Calculator is in error state."),
                "Invalid integer" => Some("Invalid integer"),
                "Cannot open Clipboard." => Some("Cannot open Clipboard."),
                "There is not enough memory for data.\rClose one or more programs, and then try again." => Some("There is not enough memory for data.\rClose one or more programs, and then try again."),
                "Not Enough Memory" => Some("Not Enough Memory"),
                "The clipboard could not be opened." => Some("The clipboard could not be opened."),
                "The clipboard could not be cleared." => Some("The clipboard could not be cleared."),
                "Not enough memory to copy the result." => Some("Not enough memory to copy the result."),
                "The clipboard memory could not be locked." => Some("The clipboard memory could not be locked."),
                "The result could not be copied to the clipboard." => Some("The result could not be copied to the clipboard."),
                "The clipboard text could not be read." => Some("The clipboard text could not be read."),
                "Clipboard integration is currently implemented for Windows only." => Some("Clipboard integration is currently implemented for Windows only."),
                "hlp-viewer.exe was not found. Place it beside the Calculator executable." => Some("hlp-viewer.exe was not found. Place it beside the Calculator executable."),
                "hlp-viewer was not found. Place the native executable beside OpenCalc." => Some("hlp-viewer was not found. Place the native executable beside OpenCalc."),
                "CALC.HLP was not found. Place CALC.HLP beside Calculator (or in the current directory); the Windows HELP directory is also checked." => Some("CALC.HLP was not found. Place CALC.HLP beside Calculator (or in the current directory); the Windows HELP directory is also checked."),
                "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc." => Some("The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc."),
                _ => None,
            },
            Language::Portuguese => match message {
                "Cannot divide by zero." => Some("Não é possível dividir por zero."),
                "Invalid input for function." => Some("Entrada inválida para a função."),
                "Result of function is undefined." => Some("O resultado da função é indefinido."),
                "Result is too large." => Some("O resultado é grande demais."),
                "Result is too small." => Some("O resultado é pequeno demais."),
                "Missing digits after numeric base prefix." => Some("Faltam dígitos após o prefixo da base numérica."),
                "Invalid based integer" => Some("Inteiro com base inválido"),
                "Missing identifier" => Some("Identificador ausente"),
                "Invalid number." => Some("Número inválido."),
                "Invalid number" => Some("Número inválido"),
                "Missing closing parenthesis." => Some("Falta o parêntese de fechamento."),
                "Unknown operator." => Some("Operador desconhecido."),
                "Calculator is in error state." => Some("A calculadora está em estado de erro."),
                "Invalid integer" => Some("Inteiro inválido"),
                "Cannot open Clipboard." => Some("Não foi possível abrir a Área de Transferência."),
                "There is not enough memory for data.\rClose one or more programs, and then try again." => Some("Não há memória suficiente para os dados.\rFeche um ou mais programas e tente novamente."),
                "Not Enough Memory" => Some("Memória insuficiente"),
                "The clipboard could not be opened." => Some("Não foi possível abrir a Área de Transferência."),
                "The clipboard could not be cleared." => Some("Não foi possível limpar a Área de Transferência."),
                "Not enough memory to copy the result." => Some("Não há memória suficiente para copiar o resultado."),
                "The clipboard memory could not be locked." => Some("Não foi possível bloquear a memória da Área de Transferência."),
                "The result could not be copied to the clipboard." => Some("Não foi possível copiar o resultado para a Área de Transferência."),
                "The clipboard text could not be read." => Some("Não foi possível ler o texto da Área de Transferência."),
                "Clipboard integration is currently implemented for Windows only." => Some("A integração com a Área de Transferência está implementada apenas no Windows."),
                "hlp-viewer.exe was not found. Place it beside the Calculator executable." => Some("hlp-viewer.exe não foi encontrado. Coloque-o ao lado do executável da Calculadora."),
                "hlp-viewer was not found. Place the native executable beside OpenCalc." => Some("hlp-viewer não foi encontrado. Coloque o executável nativo ao lado do OpenCalc."),
                "CALC.HLP was not found. Place CALC.HLP beside Calculator (or in the current directory); the Windows HELP directory is also checked." => Some("CALC.HLP não foi encontrado. Coloque CALC.HLP ao lado da Calculadora (ou no diretório atual); a pasta HELP do Windows também é verificada."),
                "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc." => Some("O arquivo de Ajuda do idioma selecionado não foi encontrado. Mantenha os arquivos HLP/CNT localizados na pasta Help ao lado do OpenCalc."),
                _ => None,
            },
            Language::Spanish => match message {
                "Cannot divide by zero." => Some("No se puede dividir por cero."),
                "Invalid input for function." => Some("Entrada no válida para la función."),
                "Result of function is undefined." => Some("El resultado de la función no está definido."),
                "Result is too large." => Some("El resultado es demasiado grande."),
                "Result is too small." => Some("El resultado es demasiado pequeño."),
                "Missing digits after numeric base prefix." => Some("Faltan dígitos después del prefijo de base numérica."),
                "Invalid based integer" => Some("Entero de base no válido"),
                "Missing identifier" => Some("Falta un identificador"),
                "Invalid number." => Some("Número no válido."),
                "Invalid number" => Some("Número no válido"),
                "Missing closing parenthesis." => Some("Falta el paréntesis de cierre."),
                "Unknown operator." => Some("Operador desconocido."),
                "Calculator is in error state." => Some("La calculadora está en estado de error."),
                "Invalid integer" => Some("Entero no válido"),
                "Cannot open Clipboard." => Some("No se pudo abrir el Portapapeles."),
                "There is not enough memory for data.\rClose one or more programs, and then try again." => Some("No hay suficiente memoria para los datos.\rCierre uno o más programas e inténtelo de nuevo."),
                "Not Enough Memory" => Some("Memoria insuficiente"),
                "The clipboard could not be opened." => Some("No se pudo abrir el Portapapeles."),
                "The clipboard could not be cleared." => Some("No se pudo borrar el Portapapeles."),
                "Not enough memory to copy the result." => Some("No hay memoria suficiente para copiar el resultado."),
                "The clipboard memory could not be locked." => Some("No se pudo bloquear la memoria del Portapapeles."),
                "The result could not be copied to the clipboard." => Some("No se pudo copiar el resultado al Portapapeles."),
                "The clipboard text could not be read." => Some("No se pudo leer el texto del Portapapeles."),
                "Clipboard integration is currently implemented for Windows only." => Some("La integración con el Portapapeles está implementada actualmente solo para Windows."),
                "hlp-viewer.exe was not found. Place it beside the Calculator executable." => Some("No se encontró hlp-viewer.exe. Colóquelo junto al ejecutable de la Calculadora."),
                "hlp-viewer was not found. Place the native executable beside OpenCalc." => Some("No se encontró hlp-viewer. Coloque el ejecutable nativo junto a OpenCalc."),
                "CALC.HLP was not found. Place CALC.HLP beside Calculator (or in the current directory); the Windows HELP directory is also checked." => Some("No se encontró CALC.HLP. Coloque CALC.HLP junto a la Calculadora (o en el directorio actual); también se comprueba la carpeta HELP de Windows."),
                "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc." => Some("No se encontró el archivo de Ayuda del idioma seleccionado. Mantenga los archivos HLP/CNT localizados en la carpeta Help junto a OpenCalc."),
                _ => None,
            },
        }
    }

    // Menu help strings are centralized too, even though Calculator has no status bar.

    pub const fn undo_help(self) -> &'static str {
        match self.language {
            Language::English => "Undo the last calculator action",
            Language::Portuguese => "Desfaz a última ação da calculadora",
            Language::Spanish => "Deshace la última acción de la calculadora",
        }
    }

    pub const fn redo_help(self) -> &'static str {
        match self.language {
            Language::English => "Redo the last undone calculator action",
            Language::Portuguese => "Refaz a última ação desfeita da calculadora",
            Language::Spanish => "Rehace la última acción deshecha de la calculadora",
        }
    }

    pub const fn copy_help(self) -> &'static str {
        match self.language {
            Language::English => "Copy the displayed value",
            Language::Portuguese => "Copia o valor exibido",
            Language::Spanish => "Copia el valor mostrado",
        }
    }

    pub const fn paste_help(self) -> &'static str {
        match self.language {
            Language::English => "Evaluate a pasted expression",
            Language::Portuguese => "Avalia uma expressão colada",
            Language::Spanish => "Evalúa una expresión pegada",
        }
    }

    pub const fn scientific_help(self) -> &'static str {
        match self.language {
            Language::English => "Switch to Scientific mode",
            Language::Portuguese => "Muda para o modo Científico",
            Language::Spanish => "Cambia al modo Científico",
        }
    }

    pub const fn standard_help(self) -> &'static str {
        match self.language {
            Language::English => "Switch to Standard mode",
            Language::Portuguese => "Muda para o modo Padrão",
            Language::Spanish => "Cambia al modo Estándar",
        }
    }

    pub const fn history_help(self) -> &'static str {
        match self.language {
            Language::English => "Show or hide the calculation history panel",
            Language::Portuguese => "Mostra ou oculta o painel de histórico de cálculos",
            Language::Spanish => "Muestra u oculta el panel del historial de cálculos",
        }
    }

    pub const fn language_help(self) -> &'static str {
        match self.language {
            Language::English => "Change the interface language",
            Language::Portuguese => "Altera o idioma da interface",
            Language::Spanish => "Cambia el idioma de la interfaz",
        }
    }

    pub const fn separator_help(self) -> &'static str {
        match self.language {
            Language::English => "Choose the decimal separator",
            Language::Portuguese => "Escolhe o separador decimal",
            Language::Spanish => "Elige el separador decimal",
        }
    }

    pub const fn help_topics_help(self) -> &'static str {
        match self.language {
            Language::English => "Open Calculator Help",
            Language::Portuguese => "Abre a Ajuda da Calculadora",
            Language::Spanish => "Abre la Ayuda de la Calculadora",
        }
    }
}
#[cfg(test)]
mod error_localization_tests {
    use super::*;
    use crate::errors::{
        CANNOT_OPEN_CLIPBOARD, DIVIDE_BY_ZERO, FUNCTION_UNDEFINED,
        INVALID_FUNCTION_INPUT, NOT_ENOUGH_MEMORY_FOR_DATA, RESULT_TOO_LARGE,
        RESULT_TOO_SMALL, STARTUP_NOT_ENOUGH_MEMORY,
    };

    #[test]
    fn undo_redo_shortcuts_are_localized_without_losing_accelerators() {
        for language in [Language::English, Language::Portuguese, Language::Spanish] {
            let strings = Strings::new(language);
            assert!(strings.undo().ends_with("\tCtrl+Z"));
            assert!(strings.redo().ends_with("\tCtrl+Y"));
            assert!(!strings.undo_help().is_empty());
            assert!(!strings.redo_help().is_empty());
        }
    }

    #[test]
    fn history_panel_strings_exist_in_every_language() {
        for language in [Language::English, Language::Portuguese, Language::Spanish] {
            let strings = Strings::new(language);
            assert!(!strings.history().is_empty());
            assert!(!strings.history_title().is_empty());
            assert!(!strings.clear_history().is_empty());
            assert!(!strings.history_help().is_empty());
        }
    }

    #[test]
    fn graph_panel_strings_exist_in_every_language() {
        for language in [Language::English, Language::Portuguese, Language::Spanish] {
            let strings = Strings::new(language);
            for text in [
                strings.graph(),
                strings.graph_help(),
                strings.graph_function(),
                strings.graph_plot(),
                strings.graph_reset_view(),
                strings.graph_export(),
                strings.graph_roots_not_plotted(),
                strings.graph_roots_visible(),
                strings.graph_no_roots(),
                strings.graph_infinite_roots(),
                strings.graph_roots_unreliable(),
                strings.graph_export_title(),
                strings.graph_export_error(),
                strings.graph_plot_error(),
            ] {
                assert!(!text.is_empty(), "missing graph text for {language:?}");
            }
        }
    }

    #[test]
    fn every_recovered_error_string_has_all_three_languages() {
        let messages = [
            DIVIDE_BY_ZERO,
            INVALID_FUNCTION_INPUT,
            FUNCTION_UNDEFINED,
            RESULT_TOO_LARGE,
            RESULT_TOO_SMALL,
            CANNOT_OPEN_CLIPBOARD,
            NOT_ENOUGH_MEMORY_FOR_DATA,
            STARTUP_NOT_ENOUGH_MEMORY,
        ];
        for language in [Language::English, Language::Portuguese, Language::Spanish] {
            let strings = Strings::new(language);
            for message in messages {
                assert!(strings.runtime_message(message).is_some(), "missing {language:?}: {message}");
            }
        }
    }
}

