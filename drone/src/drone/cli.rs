use super::{agent::AgentOptions, proxy::ProxyOptions};
use crate::database::DroneDatabase;
use crate::{acme::AcmeEabConfiguration, config::DroneConfig, keys::KeyCertPathPair};
use anyhow::Result;
use dis_spawner::{nats::TypedNats, types::ClusterName};

pub struct CertOptions {
    pub cluster_domain: String,
    pub nats: TypedNats,
    pub key_paths: KeyCertPathPair,
    pub email: String,
    pub acme_server_url: String,
    pub acme_eab_keypair: Option<AcmeEabConfiguration>,
}

pub struct DronePlan {
    pub proxy_options: Option<ProxyOptions>,
    pub agent_options: Option<AgentOptions>,
    pub cert_options: Option<CertOptions>,
    pub nats: Option<TypedNats>,
}

impl DronePlan {
    pub async fn from_drone_config(config: DroneConfig) -> Result<Self> {
        let nats = if let Some(nats) = config.nats {
            Some(nats.connect().await?)
        } else {
            None
        };

        let db = DroneDatabase::new(&config.db_path).await?;

        let cert_options = if let Some(acme_config) = config.acme {
            Some(CertOptions {
                acme_server_url: acme_config.server,
                email: acme_config.admin_email,
                cluster_domain: config.cluster_domain.clone(),
                key_paths: config
                    .cert
                    .clone()
                    .expect("Expected cert path configuration if ACME is provided."),
                nats: nats.clone().expect("Expected --nats-url."),
                acme_eab_keypair: acme_config.eab,
            })
        } else {
            None
        };

        let proxy_options = if let Some(proxy_config) = config.proxy {
            Some(ProxyOptions {
                cluster_domain: config.cluster_domain.clone(),
                db: db.clone(),
                bind_ip: proxy_config.bind_ip,
                bind_port: proxy_config.https_port,
                key_pair: config.cert.clone(),
            })
        } else {
            None
        };

        let agent_options = if let Some(agent_config) = config.agent {
            Some(AgentOptions {
                cluster_domain: ClusterName::new(&config.cluster_domain),
                db,
                docker_options: agent_config.docker,
                nats: nats
                    .clone()
                    .expect("Expected --nats-url for running agent."),
                ip: agent_config.ip,
            })
        } else {
            None
        };

        Ok(DronePlan {
            agent_options,
            cert_options,
            nats,
            proxy_options,
        })
    }
}