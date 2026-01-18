//! # Módulo de Cliente HTTP e Download
//!
//! Este módulo gerencia todas as operações de Outbound Traffic.
//! Encapsula a lógica de timeout, headers padrão e feedback visual de progresso.
//!
//! # Design
//! Utiliza `reqwest::blocking` para simplicidade síncrona
//! # Contratos
//!
//! - Apenas URLs HTTPS são aceitas
//! - Streaming direto para disco
//! - O ambiente é assumido como interativo (TTY) para exibição de progresso

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use std::fs::File;
use std::io::{self};
use std::path::Path;
use std::time::Duration;
use crate::errors::ApiError;



/// Constrói um Cliente HTTP com configurações padrão.
///
/// - Timeout elevado para arquivos grandes
/// - User-Agent explícito e auditável
pub fn create_http_client() -> Result<Client, ApiError> {
    Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(ApiError::NetworkError)
}

/// Realiza o download de um recurso remoto diretamente para o disco (Streaming).
///
/// ## Segurança
///
/// - Apenas URLs HTTPS são aceitas
/// - Status HTTP é validado explicitamente
///
/// ## Eficiência de Memória
///
/// Streaming direto da rede para o arquivo, com uso constante de memória.
///
/// # Arguments
///
/// * `client` - Instância reutilizável do `reqwest::Client`.
/// * `url` - URL completa do recurso.
/// * `destination` - Caminho local onde o arquivo será salvo.
///
/// # Returns
///
/// Retorna o número de bytes escritos em disco.
pub fn fetch_data_to_disk<P: AsRef<Path>>(
    client: &Client,
    url: &str,
    destination: P,
) -> Result<u64, ApiError> {
    if !url.starts_with("https://") {
        return Err(ApiError::HttpStatusError {
            status: reqwest::StatusCode::UPGRADE_REQUIRED,
            url: url.to_string(),
        });
    }

    let path = destination.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(ApiError::FileSystemError)?;
    }

    let mut response = client
        .get(url)
        .header(USER_AGENT, "data-gov-client/1.0")
        .header(ACCEPT, "*/*")
        .send()
        .map_err(ApiError::NetworkError)?;

    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::HttpStatusError {
            status,
            url: url.to_string(),
        });
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);

    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-"),
    );

    let file_name = path.file_name().unwrap_or_default().to_string_lossy();

    pb.set_message(format!("Baixando {}", file_name));

    let mut file = File::create(path).map_err(ApiError::FileSystemError)?;
    let mut stream_source = pb.wrap_read(&mut response);

    let bytes_written =
        io::copy(&mut stream_source, &mut file).map_err(ApiError::FileSystemError)?;

    if bytes_written == 0 {
        let _ = std::fs::remove_file(path);
        pb.finish_with_message(format!("Conteúdo Vazio: {}", file_name));
        return Err(ApiError::EmptyResponse);
    }

    pb.finish_with_message(format!("Download completo: {}", file_name));
    Ok(bytes_written)
}
