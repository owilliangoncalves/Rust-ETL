//! Metadata-Driven Configuration Models
//!
//! ## Visão Geral
//! Este módulo define as estruturas de dados para a configuração do pipeline ETL.
//! A arquitetura suporta a extração de metadados operacionais (como o `root_path`)
//! dissociados das definições de rotas, permitindo suporte a APIs heterogéneas.
//!
//! ## Boas Práticas
//! - **Encapsulamento**: Validações de integridade ocorrem no momento da carga.
//! - **Extensibilidade**: O uso de `flatten` permite adicionar novos metadados ao TOML
//!   sem quebrar a compatibilidade de tipos.

use crate::errors::ProcessorError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuração.
// Mapeia o namespace da API (ex: "compras_federal") para as suas configurações.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub apis: HashMap<String, ApiConfig>,
}

/// Configuração de uma Unidade de API.
#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    /// Ponto de entrada base da API (ex: https://api.gov.br)
    pub base_url: String,

    /// Dicionário de grupos de endpoints.
    pub endpoints: HashMap<String, EndpointGroup>,
}

/// Representa um grupo de recursos com metadados de processamento.
#[derive(Debug, Deserialize, Clone)]
pub struct EndpointGroup {
    /// Identifica a chave JSON que contém a lista de dados (ex: "resultado", "dados").
    /// Se None, assume que a estrutura é uma lista na raiz.
    pub root_path: Option<String>,

    /// Mapeamento dinâmico de chaves de identificação para caminhos relativos.
    /// Captura todas as chaves que não sejam metadados conhecidos (como root_path).
    #[serde(flatten)]
    pub routes: HashMap<String, String>,
}

impl Config {
    /// Carrega e valida o ficheiro de configuração TOML.
    ///
    /// # Erros
    /// Retorna `ProcessorError::Io` se o ficheiro não for encontrado ou
    /// `ProcessorError::Schema` se a estrutura for inválida.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ProcessorError> {
        let content = fs::read_to_string(path).map_err(ProcessorError::Io)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| ProcessorError::Schema(format!("Erro no TOML: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validação pós-carga (Fail-Fast).
    fn validate(&self) -> Result<(), ProcessorError> {
        for (api_name, api_cfg) in &self.apis {
            if api_cfg.base_url.is_empty() {
                return Err(ProcessorError::Schema(format!(
                    "'{}' sem base_url",
                    api_name
                )));
            }
            if api_cfg.endpoints.is_empty() {
                return Err(ProcessorError::Schema(format!(
                    "'{}' sem endpoints",
                    api_name
                )));
            }
        }
        Ok(())
    }

    /// Resolve a URL completa para um endpoint específico.
    pub fn resolve_endpoint_url(
        &self,
        api: &str,
        group: &str,
        key: &str,
    ) -> Result<String, ProcessorError> {
        let api_cfg = self
            .apis
            .get(api)
            .ok_or_else(|| ProcessorError::Schema(format!("API não encontrada: {}", api)))?;

        let group_cfg = api_cfg
            .endpoints
            .get(group)
            .ok_or_else(|| ProcessorError::Schema(format!("Grupo não encontrado: {}", group)))?;

        let route = group_cfg
            .routes
            .get(key)
            .ok_or_else(|| ProcessorError::Schema(format!("Chave não encontrada: {}", key)))?;

        Ok(self.join_urls(&api_cfg.base_url, route))
    }

    /// Concatenação segura de URLs sem barras duplicadas.
    fn join_urls(&self, base: &str, path: &str) -> String {
        let base_trimmed = base.trim_end_matches('/');
        let path_trimmed = path.trim_start_matches('/');
        format!("{}/{}", base_trimmed, path_trimmed)
    }
}
