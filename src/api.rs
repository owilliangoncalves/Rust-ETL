use crate::models::Config;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use reqwest::blocking::Client;
use indicatif::{ProgressBar, ProgressStyle};

/// Carrega as configurações do sistema a partir de um arquivo JSON.
///
/// Se um caminho personalizado for fornecido (`Some(path)`), tenta carregar dele.
/// Caso contrário (`None`), procura por "config.json" na raiz do projeto.
///
/// # Arguments
///
/// * `custom_path` - Um caminho opcional (`Option<&str>`) para o arquivo de configuração.
///
/// # Returns
///
/// Retorna a struct `Config` preenchida ou um erro se o arquivo não for encontrado/inválido.
pub fn load_config(custom_path: Option<&str>) -> Result<Config, Box<dyn Error>> {
    // 1. Resolve o caminho (User Input OU Padrão)
    let path = custom_path.unwrap_or("config.json");

    // 2. Tenta abrir o arquivo com contexto de erro
    let file = File::open(path).map_err(|e| {
        format!("Falha ao abrir o arquivo de configuração '{}': {}", path, e)
    })?;

    // 3. Lê e faz o Parse
    let reader = BufReader::new(file);

    // Captura erro de JSON inválido (ex: falta vírgula)
    let config = serde_json::from_reader(reader).map_err(|e| {
        format!("Erro de sintaxe no JSON do arquivo '{}': {}", path, e)
    })?;

    Ok(config)
}

/// Baixa o arquivo binário/texto direto para o disco sem carregar na RAM.
///
/// Esta função utiliza `std::io::copy`, que cria uma "mangueira" conectando
/// o fluxo da internet (Response) direto ao arquivo no disco (File).
/// O Rust usa um buffer interno minúsculo (geralmente 8KB) para fazer isso.
/// Você pode baixar um arquivo de 100GB usando apenas KBs de RAM.
///
/// # Arguments
/// * `client` - O cliente HTTP reutilizável.
/// * `url` - A URL completa da API.
/// * `caminho_destino` - Onde salvar o arquivo cru (ex: "data/raw_temp.json").
///
pub fn fetch_data_to_disk(client: &Client, url: &str, caminho_destino: &str) -> Result<(), Box<dyn Error>> {
    // 1. Configura e envia a requisição
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("Falha na conexão com {}: {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("Erro HTTP ao baixar {}: {}", url, e))?;

    // 2. Prepara a Barra de Progresso
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);

    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    pb.set_message(format!("Baixando {}", caminho_destino));

    // 3. Cria o arquivo
    let mut arquivo_destino = File::create(caminho_destino)
        .map_err(|e| format!("Não foi possível criar o arquivo '{}': {}", caminho_destino, e))?;

    // 4. Stream: Rede -> Barra -> Disco
    let mut source = pb.wrap_read(&mut response);
    io::copy(&mut source, &mut arquivo_destino)
        .map_err(|e| format!("Erro durante a escrita no disco para '{}': {}", caminho_destino, e))?;

    pb.finish_with_message(format!("Download concluído: {}", caminho_destino));

    Ok(())
}