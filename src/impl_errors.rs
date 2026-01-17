//! Implementações de traits para os enums de erro do sistema
//!
//! Este módulo existe exclusivamente para desacoplar:
//! - definição de erros (enums)
//! - implementação de traits (`Display`, `Error`, `From`)
//!
//! Segue SRP, Extreme Programming e facilita manutenção/testes.

use std::error::Error as StdError;
use std::fmt;

use crate::errors::ProcessorError;

/* ========================================================================== */
/* Display                                                                    */
/* ========================================================================== */

impl fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessorError::Io(err) => {
                write!(f, "[I/O] {}", err)
            }

            ProcessorError::Json(err) => {
                write!(f, "[JSON] {}", err)
            }

            ProcessorError::Parquet(err) => {
                write!(f, "[Parquet] {}", err)
            }

            ProcessorError::Schema(msg) => {
                write!(f, "[Schema] {}", msg)
            }
        }
    }
}

/* ========================================================================== */
/* std::error::Error                                                          */
/* ========================================================================== */

impl StdError for ProcessorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ProcessorError::Io(err) => Some(err),
            ProcessorError::Json(err) => Some(err),
            ProcessorError::Parquet(err) => Some(err),
            ProcessorError::Schema(_) => None,
        }
    }
}

/* ========================================================================== */
/* Conversions                                                                */
/* ========================================================================== */

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

impl From<parquet::errors::ParquetError> for ProcessorError {
    fn from(err: parquet::errors::ParquetError) -> Self {
        ProcessorError::Parquet(err)
    }
}

/* ========================================================================== */
/* Box<dyn Error>                                                             */
/* ========================================================================== */

impl From<ProcessorError> for Box<dyn StdError> {
    fn from(err: ProcessorError) -> Self {
        Box::new(err)
    }
}
