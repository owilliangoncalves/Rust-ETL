use polars::prelude::*;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor};

/// Processa um arquivo JSON bruto contendo um envelope da API e converte os dados extraídos para Parquet.
///
/// Converte os dados para um DataFrame do Polars e os salva no disco com compressão ZSTD.
///
/// # Arguments
///
/// * `input_path` - O caminho do arquivo JSON bruto (ex: `data/raw_temp.json`).
/// * `output_path` - O caminho onde o arquivo Parquet final será salvo.
///
/// # Returns
///
/// Retorna uma tupla `(usize, usize)` contendo o número de **(linhas, colunas)** do DataFrame gerado.
/// Retorna `(0, 0)` se a lista de resultados estiver vazia.
///
/// # Errors
///
/// Esta função retornará um erro se:
///
/// * O arquivo de entrada não puder ser aberto ou lido.
/// * O JSON for inválido ou não contiver o campo `resultado`.
/// * O campo `resultado` não for uma lista (Array).
/// * Ocorrer um erro interno no Polars durante a inferência de tipos ou escrita do Parquet.
///
pub fn process_raw_to_parquet(
    input_path: &str,
    output_path: &str,
) -> Result<(usize, usize), Box<dyn Error>> {
    // 1. Leitura Bufferizada
    let file = File::open(input_path).map_err(|e| format!("Erro ao abrir arquivo bruto: {}", e))?;
    let reader = BufReader::new(file);

    // 2. Parse do JSON Envelope
    let json_envelope: Value = serde_json::from_reader(reader)?;

    // 3. Extração Segura do Array
    let lista_dados = json_envelope
        .get("resultado")
        .ok_or("Campo 'resultado' ausente no JSON.")?;

    let array_dados = lista_dados
        .as_array()
        .ok_or("O campo 'resultado' não é uma lista válida.")?;

    // Proteção contra lista vazia
    if array_dados.is_empty() {
        // Retornamos 0,0 para que o main saiba que nada foi gerado
        return Ok((0, 0));
    }

    // 4. Preparação para o Polars
    // usando to_vec (bytes)
    let json_bytes = serde_json::to_vec(array_dados)?;
    let cursor = Cursor::new(json_bytes);

    // 5. Criação do DataFrame
    let mut df = JsonReader::new(cursor)
        // None = Lê a página inteira
        .infer_schema_len(None)
        .finish()
        .map_err(|e| format!("Erro ao converter JSON para DataFrame: {}", e))?;

    let shape = df.shape(); // Captura (linhas, colunas)

    // 6. Salvando em Parquet com Compressão (Storage Optimization)
    let file_out = File::create(output_path)?;

    // Compressão ZSTD
    ParquetWriter::new(file_out)
        .with_compression(ParquetCompression::Zstd(None))
        .finish(&mut df)?;

    Ok(shape)
}
