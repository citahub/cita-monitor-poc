use cratedb::Cluster;
use cratedb::sql::QueryRunner;
use std::thread;
use std::time::Duration;
use tokio_core::reactor::Core;
use web3::{self, Web3};
use web3::types::{BlockId, BlockNumber};

static INSERT_STMT: &str = "insert into blocks(height, txs_num, quota_used, quota_per_tx) values (?, ?, ?, ?)";

pub fn start(pull_interval: u64, jsonrpc_url: String, cratedb_url: String) {
    thread::spawn(move || {
        let mut event_loop = Core::new().unwrap();
        let web3_client =
            Web3::new(web3::transports::Http::with_event_loop(&jsonrpc_url, &event_loop.handle(), 1).unwrap());
        let cita_client = web3_client.cita();
        let cratedb_client = Cluster::from_string(cratedb_url).unwrap();
        // get current block number
        let call = cita_client.block_number();
        let mut height = event_loop.run(call).unwrap().low_u64();
        info!("start get block metrics, current height: {}", height);

        loop {
            let call = cita_client.block(BlockId::from(BlockNumber::Number(height)));
            match event_loop.run(call) {
                Ok(block) => {
                    assert_eq!(block.header.number.low_u64(), height);
                    let quota_used = block.header.gas_used.low_u64();
                    let txs_num = block.body.transactions.len() as u64;
                    let quota_per_tx = if txs_num == 0 {
                        0 as u64
                    } else {
                        (quota_used / txs_num) as u64
                    };
                    let value = Box::new((height, txs_num, quota_used, quota_per_tx));
                    let _ = cratedb_client.query(INSERT_STMT, Some(value)).unwrap();
                    info!("insert block {} metrics ok", height);
                    height += 1;
                }
                // TODO: error handle
                Err(_err) => info!("get block {} stuck", height),
            }
            thread::sleep(Duration::from_millis(pull_interval));
        }
    });
}
