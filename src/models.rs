use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Representa a configuração global do pipeline de dados.
///
/// Esta estrutura é responsável por mapear o arquivo `endpoints.json` para objetos em memória.
/// Ela atua como a "Fonte da Verdade" para as URLs e recursos que serão processados.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// A URL base (host + path inicial) da API.
    pub base_url: String,

    /// Um dicionário (Mapa) de endpoints a serem processados.
    ///
    /// * **Chave (Key):** O nome amigável do recurso. Será usado para nomear os arquivos
    ///   de saída (ex: `raw_compras.json`, `compras.parquet`).
    /// * **Valor (Value):** O caminho relativo (sufixo) da URL, incluindo query strings.
    pub endpoints: HashMap<String, String>,
}
