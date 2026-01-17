mod analysis;
mod api;
mod models;

use reqwest::blocking::Client;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Orquestra a execução do pipeline de dados (ETL).
///
/// Esta função gerencia o ciclo de vida completo dos dados:
/// 1. **Setup:** Validação de diretórios e carregamento de configuração.
/// 2. **Ingestão (Extract):** Download via stream para disco (Zero RAM overhead).
/// 3. **Transformação (Transform):** Conversão de JSON bruto para Parquet com compressão.
/// 4. **Limpeza (Cleanup):** Remoção de artefatos temporários.
///
/// # Usage
///
/// O programa aceita um argumento opcional via linha de comando para especificar
/// o arquivo de configuração.
///
/// ```bash
/// # Usa o padrão 'endpoints.json'
/// cargo run
///
/// # Usa um arquivo específico
/// cargo run -- config_prod.json
/// ```
///
/// # Errors
///
/// A função retornará um erro (`Box<dyn Error>`) se:
///
/// * Ocorrer falha na criação do diretório `data`.
/// * O arquivo de configuração não for encontrado ou contiver JSON inválido.
/// * Ocorrerem erros fatais de I/O (ex: disco cheio, sem permissão).
///
/// *Nota: Erros individuais de download ou conversão de um endpoint específico
/// são logados no console, mas não interrompem a execução dos demais.*
fn main() -> Result<(), Box<dyn Error>> {
    let inicio_global = Instant::now();

    println!("--- 🚀 INICIANDO PIPELINE DE DADOS USANDO RUST E POLARS 🐻‍❄️ ---");

    // Definição do diretório de saída
    let output_dir = Path::new("data");

    // 1. Setup: Garante diretório de saída
    if !output_dir.exists() {
        println!(" -> Criando diretório de saída '{:?}'...", output_dir);
        fs::create_dir(output_dir)?;
    }

    // 2. Configuração: Leitura via CLI ou Padrão
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str());

    let config = api::load_config(config_path)?;
    println!(" -> Configuração carregada. Base URL: {}", config.base_url);

    // 3. Otimização: Client HTTP Keep-Alive
    let client = Client::new();

    // 4. Loop de Processamento
    for (nome, endpoint) in &config.endpoints {
        let url = format!("{}{}", config.base_url, endpoint);

        println!("\n==========================================");
        println!("PROCESSANDO: {}", nome);

        // Construção segura de caminhos (PathBuf)
        let caminho_raw: PathBuf = output_dir.join(format!("raw_{}.json", nome));
        let caminho_parquet: PathBuf = output_dir.join(format!("{}.parquet", nome));

        // Conversão para str para uso nas funções (unwrap seguro pois definimos os nomes acima)
        let raw_str = caminho_raw.to_str().unwrap();
        let parquet_str = caminho_parquet.to_str().unwrap();

        // --- INGESTÃO ---
        println!(" 1. 📥 Baixando stream...");
        match api::fetch_data_to_disk(&client, &url, raw_str) {
            Ok(_) => println!("    [OK] Download concluído."),
            Err(e) => {
                eprintln!("    [ERRO] Falha no download: {}", e);
                continue; // Fail-soft: Pula para o próximo item
            }
        }

        // --- TRANSFORMAÇÃO ---
        println!(" 2. ⚙️ Convertendo para Parquet...");

        // Verificação de arquivo vazio
        if fs::metadata(&caminho_raw)?.len() == 0 {
            eprintln!("    [AVISO] Arquivo vazio baixado. Ignorando.");
            let _ = fs::remove_file(&caminho_raw);
            continue;
        }

        match analysis::process_raw_to_parquet(raw_str, parquet_str) {
            Ok((linhas, colunas)) => {
                if linhas == 0 {
                    println!("    ⚠️  Arquivo gerado sem dados (Lista vazia).");
                } else {
                    println!("    [OK] Arquivo salvo: {}", parquet_str);
                    println!("    Shape: {} linhas x {} colunas", linhas, colunas);
                }

                // --- LIMPEZA ---
                println!(" 3. 🧹 Limpando temporários...");
                if let Err(e) = fs::remove_file(&caminho_raw) {
                    eprintln!("    [AVISO] Falha ao limpar temp: {}", e);
                }
            }
            Err(e) => eprintln!("    [ERRO] Falha crítica na conversão: {}", e),
        }
    }

    let duracao = inicio_global.elapsed();
    println!("\n==========================================");
    println!("✅ PIPELINE FINALIZADO!");
    println!("⏱️  Tempo Total: {:.2?}", duracao);

    Ok(())
}
