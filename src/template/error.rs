/// Errors that can occur during template operations.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Template failed to parse.
    #[error("template parse error: {0}")]
    ParseError(String),

    /// Template failed to render.
    #[error("template render error: {0}")]
    RenderError(String),

    /// YAML-to-Liquid value conversion failed.
    #[error("conversion error: {0}")]
    ConversionError(String),

    /// I/O error reading layout or include files.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Layout not found.
    #[error("layout not found: {0}")]
    LayoutNotFound(String),
}
