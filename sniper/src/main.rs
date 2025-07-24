use sniper::JitoClient;
// 引入所需的库
use std::io::Error;

fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    // 启动客户端逻辑
    JitoClient::start()
}
