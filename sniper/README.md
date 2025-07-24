# 部署

1. 复制 `[.env.example](.env.example)` 文件, 改名为 `.env`
2. 修改配置
    1. 更新 NONCE_KEY, PUBLIC_KEY, PRIVATE_KEY 为自己钱包的信息
    2. JITO_RPC_ENDPOINTS 和 ZERO_SLOT_RPC_ENDPOINTS 根据自己的情况替换
3. (可选) 如果有私有 rpc 和 grpc 可以修改 RPC_ENDPOINTS 和 YELLOWSTONE_GRPC_URL
4. (可选) 修改手续费要通过修改代码


# 运行测试

```shell
# 测试 process entries
cargo test tests::test_process_entries -- --nocapture

# 测试 yellowstone listener
cargo test tests::test_yellowstone_listener -- --nocapture
```