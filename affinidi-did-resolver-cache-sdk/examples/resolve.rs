use affinidi_did_resolver_cache_sdk::{
    config::ClientConfigBuilder, errors::DIDCacheError, DIDCacheClient,
};
use clap::Parser;
use tracing_subscriber::filter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// network address if running in network mode (ws://127.0.0.1:8080/did/v1/ws)
    #[arg(short, long)]
    network_address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), DIDCacheError> {
    // **************************************************************
    // *** Initial setup
    // **************************************************************
    let args = Args::parse();

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .with_env_filter(filter::EnvFilter::from_default_env())
        .finish();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).expect("Logging failed, exiting...");

    // test did
    let peer_did = "did:peer:2.Vz6MkiToqovww7vYtxm1xNM15u9JzqzUFZ1k7s7MazYJUyAxv.EzQ3shQLqRUza6AMJFbPuMdvFRFWm1wKviQRnQSC1fScovJN4s.SeyJ0IjoiRElEQ29tbU1lc3NhZ2luZyIsInMiOnsidXJpIjoiaHR0cHM6Ly8xMjcuMC4wLjE6NzAzNyIsImEiOlsiZGlkY29tbS92MiJdLCJyIjpbXX19";

    println!();
    println!(" ****************************** ");
    println!(" *  Local Resolver Example    * ");
    println!(" ****************************** ");
    println!();

    // Create a new local client configuration, use default values
    let local_config = ClientConfigBuilder::default().build();
    let local_resolver = DIDCacheClient::new(local_config).await?;
    match local_resolver.resolve(peer_did).await {
        Ok(response) => {
            println!(
                "Resolved DID ({}) did_hash({}) Document:\n{:#?}\n",
                response.did, response.did_hash, response.doc
            );
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" ****************************** ");
    println!();
    // create a network client configuration, set the service address.
    let mut network_config = ClientConfigBuilder::default()
        .with_cache_ttl(60) // Change the cache TTL to 60 seconds
        .with_network_timeout(20_000); // Change the network timeout to 20 seconds
    if let Some(address) = &args.network_address {
        println!("Running in network mode with address: {}", address);
        network_config = network_config.with_network_mode(address);
    } else {
        println!("Running in local mode.");
    }

    let network_resolver = DIDCacheClient::new(network_config.build()).await?;
    let response = network_resolver.resolve(peer_did).await?;

    println!("Resolved DID Document: {:#?}", response.doc);

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" *  did:key method            * ");
    println!(" ****************************** ");
    println!();
    let response = network_resolver
        .resolve("did:key:z6MkiToqovww7vYtxm1xNM15u9JzqzUFZ1k7s7MazYJUyAxv")
        .await?;
    println!("Resolved DID Document: {:#?}", response.doc);

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" *  did:web method            * ");
    println!(" ****************************** ");
    println!();
    let response = network_resolver.resolve("did:web:affinidi.com").await?;
    println!("Resolved DID Document: {:#?}", response.doc);

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" *  did:ethr method           * ");
    println!(" ****************************** ");
    println!();
    let response = network_resolver
        .resolve("did:ethr:0x1:0xb9c5714089478a327f09197987f16f9e5d936e8a")
        .await?;
    println!("Resolved DID Document: {:#?}", response.doc);

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" *  did:jwk method            * ");
    println!(" ****************************** ");
    println!();
    let response = network_resolver
        .resolve("did:jwk:eyJjcnYiOiJQLTI1NiIsImt0eSI6IkVDIiwieCI6ImFjYklRaXVNczNpOF91c3pFakoydHBUdFJNNEVVM3l6OTFQSDZDZEgyVjAiLCJ5IjoiX0tjeUxqOXZXTXB0bm1LdG00NkdxRHo4d2Y3NEk1TEtncmwyR3pIM25TRSJ9")
        .await?;
    println!("Resolved DID Document: {:#?}", response.doc);

    println!();
    println!(" ****************************** ");
    println!(" *  Network Resolver Example  * ");
    println!(" *  did:pkh method            * ");
    println!(" ****************************** ");
    println!();
    let response = network_resolver
        .resolve(
            "did:pkh:solana:4sGjMW1sUnHzSxGspuhpqLDx6wiyjNtZ:CKg5d12Jhpej1JqtmxLJgaFqqeYjxgPqToJ4LBdvG9Ev",
        )
        .await?;
    println!("Resolved DID Document: {:#?}", response.doc);

    Ok(())
}
