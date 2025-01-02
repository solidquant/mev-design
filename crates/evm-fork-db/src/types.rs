use std::path::Path;
use std::sync::Arc;

use reth_chainspec::ChainSpecBuilder;
use reth_db::{open_db_read_only, DatabaseEnv};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::providers::StaticFileProvider;
use reth_provider::ProviderFactory;

pub type DBFactory = ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>;

pub fn get_db(db_path: &str) -> DatabaseEnv {
    let db_path = Path::new(db_path);
    open_db_read_only(&db_path, Default::default()).unwrap()
}

pub fn get_db_factory(db_path: &str, static_path: &str) -> DBFactory {
    let db = get_db(db_path);
    let spec = ChainSpecBuilder::mainnet().build();

    ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
        db.into(),
        spec.into(),
        StaticFileProvider::read_only(static_path, true).unwrap(),
    )
}
