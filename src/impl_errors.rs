//! # Implementações de Traits de Erro
//!
//! Contém a lógica de exibição, hierarquia e conversão de erros.
//!
//! ## Decisões de Arquitetura
//! - Implementamos `std::fmt::Display` para logs amigáveis.
//! - Implementamos `std::error::Error` para compatibilidade com `anyhow` ou `Box<dyn Error>`.
//! - Implementamos `From<T>` para permitir coerção automática via operador `?`.

use std::error::Error as StdError;
use std::fmt;

// Importação do Enum de erros e da biblioteca Polars (necessária para o ETL)
use crate::errors::ProcessorError;
use polars::error::PolarsError;

// Display
impl fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessorError::Io(err) => write!(f, "Falha de sistema no processo de IO: {}", err),
            ProcessorError::Json(err) => write!(f, "Falha de parsing do .json: {}", err),
            ProcessorError::Parquet(msg) => write!(f, "Erro de processamento em parquet: {}", msg),
            ProcessorError::Schema(msg) => write!(f, "Violação de regra no .toml: {}", msg),
        }
    }
}


// StdError
impl StdError for ProcessorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            // Delega para a implementação interna do erro original
            ProcessorError::Io(err) => Some(err),
            ProcessorError::Json(err) => Some(err),
            ProcessorError::Parquet(_) => None,
            ProcessorError::Schema(_) => None,
        }
    }
}


//Conversões Automáticas (From)
impl From<std::io::Error> for ProcessorError {
    fn from(err: std::io::Error) -> Self {
        ProcessorError::Io(err)
    }
}

impl From<serde_json::Error> for ProcessorError {
    fn from(err: serde_json::Error) -> Self {
        ProcessorError::Json(err)
    }
}

// Conversão para PolarsError.
impl From<PolarsError> for ProcessorError {
    fn from(err: PolarsError) -> Self {
        ProcessorError::Parquet(err.to_string())
    }
}
