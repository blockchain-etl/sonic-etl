#[derive(Debug, Clone)]
pub struct TransformationErr {
    err: String,
    field: Option<String>,
}

impl TransformationErr {
    #[inline]
    pub fn new(err: String, field: Option<String>) -> Self {
        Self {
            err: err.to_string(),
            field: field.map(|item| item.to_string()),
        }
    }
}

impl std::fmt::Display for TransformationErr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.field {
            Some(field_name) => write!(
                f,
                "Error during transformation with field `{}`: {}",
                field_name, self.err
            ),
            None => write!(f, "Error during transformation: {}", self.err),
        }
    }
}

impl std::error::Error for TransformationErr {}
