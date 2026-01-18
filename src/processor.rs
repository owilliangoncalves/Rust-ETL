//! # JSON to Parquet Normalization Engine
//!
//! ## Engenharia de Dados
//! Este módulo implementa a normalização dinâmica de JSON para Parquet.

use polars::io::SerReader;
use polars::prelude::StatisticsOptions;
use polars::prelude::*;

use std::fs::{self, File};
use std::num::NonZeroUsize;
use std::path::Path;

use crate::errors::ProcessorError;

/// Converte JSON para Parquet utilizando metadados de root_path.
pub fn process_json_to_parquet(
    json_path: &Path,
    parquet_path: &Path,
    root_path: Option<&str>,
) -> Result<(), ProcessorError> {
    // Abertura do arquivo original
    let file = File::open(json_path).map_err(ProcessorError::Io)?;
    let schema_len = NonZeroUsize::new(1000).unwrap();

    let mut dataframe = JsonReader::new(file)
        .infer_schema_len(Some(schema_len))
        .finish()
        .map_err(|e| ProcessorError::Parquet(format!("Falha no parsing JSON: {}", e)))?;

    if dataframe.height() == 0 {
        return Err(ProcessorError::Schema(
            "Arquivo JSON sem registros ou vazio".to_string(),
        ));
    }

    // Normalização Dinâmica
    if let Some(path) = root_path
        && !path.is_empty()
        && dataframe.column(path).is_ok()
    {
        let dtype = dataframe.column(path)?.dtype();

        match dtype {
            DataType::List(_) => {
                dataframe = dataframe.explode([path])?.unnest([path])?;
            }
            DataType::Struct(_) => {
                dataframe = dataframe.unnest([path])?;
            }
            _ => {
                dataframe = dataframe.unnest([path]).unwrap_or(dataframe);
            }
        }
    }

    // Limpeza de Colunas Técnicas
    let technical_cols = [
        "totalRegistros",
        "totalPaginas",
        "paginasRestantes",
        "links",
        "dataHoraConsulta",
        "timeZoneAtual",
        "dataHoraAtualizacao",
    ];

    for col in technical_cols {
        if dataframe.column(col).is_ok() {
            dataframe = dataframe.drop(col)?;
        }
    }

    // Sanitização de Encodings
    dataframe = byte_arrays(dataframe)?;

    // Escrita Parquet (Escopo corrigido)
    let file_out = File::create(parquet_path).map_err(ProcessorError::Io)?;

    let stats_options = StatisticsOptions {
        min_value: true,
        max_value: true,
        null_count: true,
        distinct_count: false,
    };

    ParquetWriter::new(file_out)
        .with_compression(ParquetCompression::Snappy)
        .with_statistics(stats_options)
        .finish(&mut dataframe) // Referência mutável para a variável 'dataframe'
        .map_err(|e| ProcessorError::Parquet(format!("Erro ao gravar Parquet: {}", e)))?;

    // Finalização
    fs::remove_file(json_path).map_err(ProcessorError::Io)?;

    Ok(())
}

/// Converte List<Int64> (ASCII) para String legível.
fn byte_arrays(mut df_internal: DataFrame) -> Result<DataFrame, ProcessorError> {
    let col_names = df_internal.get_column_names_owned();

    for name_smart in col_names {
        let name_str = name_smart.as_str();
        let s = df_internal.column(name_str)?;

        if let DataType::List(inner_type) = s.dtype()
            && matches!(**inner_type, DataType::Int64 | DataType::Float64)
        {
            let clean_series = s
                .clone()
                .cast(&DataType::List(Box::new(DataType::UInt8)))?
                .list()?
                .apply_to_inner(&|inner: Series| -> PolarsResult<Series> {
                    inner.cast(&DataType::Binary)
                })?
                .cast(&DataType::String)?;

            df_internal.replace(name_str, clean_series)?;
        }
    }
    Ok(df_internal)
}
