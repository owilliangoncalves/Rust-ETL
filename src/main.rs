//! # Metadata-Driven Data Pipeline
//!
//! ## Visão Geral
//! Extração de dados de múltiplas APIs governamentais, aplicando
//! transformações normalizadas via Polars com base em configurações dinâmicas.
//!
//! ## Princípios de Engenharia
//! - **Resiliência (Fail-Soft)**: Erros individuais em endpoints não abortam o pipeline.
//! - **Observabilidade**: Logs detalhados com tempos de execução por etapa.
//! - **Atomização**: Garantia de que arquivos temporários sejam limpos apenas após o sucesso.

mod api;
mod errors;
mod impl_errors;
mod models;
mod processor;

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::models::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let global_timer = Instant::now();

    println!("--- INICIANDO ETL PIPELINE ---");

    // Define o diretório base para armazenamento físico
    let data_root = Path::new("data");
    if !data_root.exists() {
        fs::create_dir_all(data_root)?;
    }

    // Carrega configuração TOML (permite passar caminho via CLI)
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "endpoints_publicos.toml".to_string());

    let config = match Config::load_from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Erro na carga de configuração: {}", e);
            std::process::exit(1);
        }
    };

    // Reuso de conexões/Keep-alive para performance
    let client = api::create_http_client()?;

    for (api_name, api_config) in &config.apis {
        println!("\n Domínio: {}", api_name.to_uppercase());

        for (group_name, group_config) in &api_config.endpoints {
            println!("Grupo: {}", group_name);

            // Resgata metadado de normalização (root_path) do TOML
            let root_path = group_config.root_path.as_deref();

            // Garante estrutura de pastas: data/{api}/{grupo}
            let group_dir = data_root.join(api_name).join(group_name);
            fs::create_dir_all(&group_dir)?;

            // Itera sobre as rotas dinâmicas capturadas pelo flatten
            for key in group_config.routes.keys() {
                let step_timer = Instant::now();

                // Resolve URL completa
                let url = match config.resolve_endpoint_url(api_name, group_name, key) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("Erro ao resolver URL para '{}': {}", key, e);
                        continue;
                    }
                };

                // Pula endpoints que exigem substituição manual de ID {id}
                if url.contains('{') {
                    continue;
                }

                // Definição de caminhos físicos
                let path_json = group_dir.join(format!("{}_temp.json", key));
                let path_parquet = group_dir.join(format!("{}.parquet", key));

                println!("Processando: {}", key);

                if let Err(e) = api::fetch_data_to_disk(&client, &url, &path_json) {
                    eprintln!("Falha no Download: {}", e);
                    continue;
                }

                // Injeta o root_path específico de cada órgão/grupo
                match processor::process_json_to_parquet(&path_json, &path_parquet, root_path) {
                    Ok(_) => {
                        println!(
                            "Sucesso: Parquet gerado ({:.2?})",
                            step_timer.elapsed()
                        );
                    }
                    Err(e) => {
                        eprintln!("Falha na Transformação: {}", e);
                    }
                }
            }
        }
    }

    println!("\n==========================================");
    println!("Fim da extração e conversão de dados");
    println!(
        "Tempo de execução: {:.2?}",
        global_timer.elapsed()
    );
    println!("==========================================");

    Ok(())
}
