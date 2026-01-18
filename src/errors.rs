//! # Definição de Erros do Domínio de Processamento
//!
//! Este módulo centraliza possíveis durante a etapa de transformação (ETL).
//!
//! # Error Handling Strategy
//! - **Tipagem:** Enums para tratamento exaustivo.
//! - **Extensibilidade:** Marcado como `non_exhaustive` para permitir evolução sem quebra de contrato.

/// Enumeração central de falhas do Processador.
///
/// O atributo `#[non_exhaustive]` garante compatibilidade futura,
/// instruindo o compilador a exigir tratamento de variantes desconhecidas.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProcessorError {
    /// Falhas no sistema de arquivos (permissão, disco cheio, arquivo inexistente).
    /// Encapsula `std::io::Error`.
    Io(std::io::Error),

    /// Encapsula `serde_json::Error`.
    Json(serde_json::Error),

    /// Erros originados na engine.
    /// Armazenados como `String` para reduzir acoplamento direto.
    Parquet(String),

    /// Violações de regras de negócio ou inconsistência de formato nos dados (ex: Schema mismatch).
    Schema(String),
}
/// Define erros específicos da camada de API/Rede.
#[derive(Debug)]
pub enum ApiError {
    /// Falha na conexão, DNS ou handshake TLS.
    NetworkError(reqwest::Error),

    /// O servidor respondeu, mas com status HTTP de erro.
    HttpStatusError {
        status: reqwest::StatusCode,
        url: String,
    },

    /// Falha ao criar diretórios ou escrever no disco.
    FileSystemError(std::io::Error),

    /// O servidor respondeu com sucesso, mas nenhum byte útil foi recebido.
    EmptyResponse,
}