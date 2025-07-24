use analyzer_protos::shared::whitelist_service_server::{WhitelistService, WhitelistServiceServer};
use analyzer_protos::shared::{WhitelistItem, WhitelistRequest, WhitelistResponse};
use sqlx::{PgPool, Pool, Postgres, Row};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tonic::codegen::Body;
use tonic::transport::Error;
use tonic::{Request, Response, Status};

const CREATOR_COUNT_SQL: &str =
    r#"SELECT COUNT(DISTINCT token_creator) AS total_creators FROM token_states"#;
const MINT_COUNT_SQL: &str =
    r#"SELECT COUNT(DISTINCT token_address) AS total_tokens FROM token_states"#;
const WHITELIST_SQL: &str = r#"
WITH token_stats AS (SELECT token_creator,
                            COUNT(DISTINCT token_address)                                                      AS token_count,
                            CAST(AVG(dev_holding_duration) AS FLOAT8)                                          AS avg_holding_seconds,
                            CAST(SUM(dev_profit) AS FLOAT8)                                                    AS total_profit_sol,
                            SUM(CASE WHEN dev_holding_duration <= 5 THEN 1 ELSE 0 END)                         AS hold_less_5_sec_count,
                            SUM(CASE WHEN dev_holding_duration > 5 THEN 1 ELSE 0 END)                          AS hold_greater_5_sec_count,
                            SUM(CASE
                                    WHEN dev_holding_duration > 5 AND dev_holding_duration < 10 THEN 1
                                    ELSE 0 END)                                                                AS mid_hold_count,
                            MIN(dev_holding_duration)                                                          AS min_holding_seconds,
                            CAST(SUM(CASE WHEN dev_profit > 0 THEN 1 ELSE 0 END) * 100.0 / COUNT(*) AS FLOAT8) AS win_rate,
                            MAX(dev_holding_start_time)                                                        AS latest_trade_time,
                            CAST(SUM(CASE WHEN dev_profit > 0 THEN dev_profit ELSE 0 END) AS FLOAT8)           AS positive_dev_profit,
                            CAST(SUM(CASE WHEN dev_initial_buy > 0 THEN dev_initial_buy ELSE 0 END) AS BIGINT) AS positive_dev_initial_buy
                     FROM token_states
                     WHERE dev_profit IS NOT NULL
                       AND dev_initial_buy IS NOT NULL
                     GROUP BY token_creator),
     user_counts AS (SELECT ts.token_creator,
                            CAST(AVG(uc.user_count) AS FLOAT8) AS avg_users_per_token
                     FROM token_states ts
                              JOIN (SELECT token_address,
                                           COUNT(DISTINCT useraddr) AS user_count
                                    FROM token_trades
                                    GROUP BY token_address) uc ON ts.token_address = uc.token_address
                     GROUP BY ts.token_creator),
     top3_buyers_avg AS (SELECT token_creator,
                                AVG(top3.sol_total / 1000000000.0) AS avg_top3_buy
                         FROM (SELECT ts.token_creator,
                                      ts.token_address,
                                      SUM(tr.sol_amount) AS sol_total
                               FROM token_states ts
                                        JOIN (SELECT token_address,
                                                     sol_amount,
                                                     ROW_NUMBER() OVER (PARTITION BY token_address ORDER BY timestamp ASC) AS rn
                                              FROM token_trades
                                              WHERE is_buy = TRUE) tr ON ts.token_address = tr.token_address
                               WHERE tr.rn <= 3
                               GROUP BY ts.token_creator, ts.token_address) top3
                         GROUP BY token_creator)
SELECT ts.*,
       CAST((ts.positive_dev_profit / (ts.positive_dev_initial_buy / 1000000000.0)) * 100 AS FLOAT8) AS profitability,
       COALESCE(uc.avg_users_per_token, 0)                                                           AS avg_users_per_token,
       CAST(COALESCE(tb.avg_top3_buy, 0) AS FLOAT8)                                                  AS avg_top3_buy_per_token
FROM token_stats ts
         LEFT JOIN user_counts uc ON ts.token_creator = uc.token_creator
         LEFT JOIN top3_buyers_avg tb ON ts.token_creator = tb.token_creator
    WHERE
        avg_holding_seconds > $1
      AND total_profit_sol > $2
      AND token_count > $3
      AND mid_hold_count <= $4
      AND hold_less_5_sec_count <= $5
      AND min_holding_seconds >= $6
      AND avg_users_per_token >= $7
      AND COALESCE(tb.avg_top3_buy, 0) >= $8
    ORDER BY total_profit_sol DESC
"#;

pub struct MyService {
    pool: Arc<Pool<Postgres>>,
}

#[tonic::async_trait]
impl WhitelistService for MyService {
    async fn get_whitelist(
        &self,
        request: Request<WhitelistRequest>,
    ) -> Result<Response<WhitelistResponse>, Status> {
        let param = request.into_inner();

        let total_creators: i64 = sqlx::query(CREATOR_COUNT_SQL)
            .fetch_one(&*self.pool)
            .await
            .unwrap()
            .get("total_creators");
        
        let total_tokens: i64 = sqlx::query(MINT_COUNT_SQL)
            .fetch_one(&*self.pool)
            .await
            .unwrap()
            .get("total_tokens");
        
        let filtered_items = sqlx::query(WHITELIST_SQL)
            .bind(param.avg)
            .bind(param.profit)
            .bind(param.count)
            .bind(param.mid)
            .bind(param.hold_less_5_sec_count)
            .bind(param.min_hold)
            .bind(param.avg_user)
            .bind(param.top_3_buy)
            .fetch_all(&*self.pool)
            .await
            .unwrap();

        let mut filtered_creators = HashSet::new();

        let filtered_items = filtered_items
            .into_iter()
            .map(|row| {
                filtered_creators.insert(row.get("token_creator"));

                WhitelistItem {
                    token_creator: row.get("token_creator"),
                    token_count: row.get("token_count"),
                    avg_holding_seconds: row.get("avg_holding_seconds"),
                    total_profit_sol: row.get("total_profit_sol"),
                    hold_less_5_sec_count: row.get("hold_less_5_sec_count"),
                    hold_greater_5_sec_count: row.get("hold_greater_5_sec_count"),
                    mid_hold_count: row.get("mid_hold_count"),
                    min_holding_seconds: row.get("min_holding_seconds"),
                    win_rate: row.get("win_rate"),
                    latest_trade_time: row.get("latest_trade_time"),
                    positive_dev_profit: row.get("positive_dev_profit"),
                    positive_dev_initial_buy: row.get("positive_dev_initial_buy"),
                    profitability: row.get("profitability"),
                    avg_users_per_token: row.get("avg_users_per_token"),
                    avg_top3_buy_per_token: row.get("avg_top3_buy_per_token"),
                }
            })
            .collect();

        Ok(Response::new(WhitelistResponse {
            total_creators,
            total_tokens,
            filtered_creators: filtered_creators.into_iter().collect(),
            filtered_items,
        }))
    }
}

pub async fn start_server_thread(pool: Arc<Pool<Postgres>>) {
    let exit = AtomicBool::new(false);

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(async move {
            tonic::transport::Server::builder()
                .add_service(WhitelistServiceServer::new(MyService { pool }))
                .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8090))
                .await
                .unwrap();
        });

        while !exit.load(Ordering::Relaxed) {}
    });
}
