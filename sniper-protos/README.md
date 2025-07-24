# 测试
```shell
grpcurl -plaintext -proto ./proto/config.proto -d '{}' 127.0.0.1:9090 config.ConfigService/GetConfig

grpcurl -plaintext -proto ./proto/config.proto -d '{
                                                     "maxSol": 0.02,
                                                     "jitoFee": 0.00013,
                                                     "zeroSlotBuyFee": 0.00013,
                                                     "zeroSlotSellFee": 0.0001
                                                   }
' 127.0.0.1:9090 config.ConfigService/UpdateConfig

grpcurl -plaintext -proto ./proto/config.proto -d '{}' 127.0.0.1:9090 config.ConfigService/GetBlacklist

grpcurl -plaintext -proto ./proto/config.proto -d '{"item": "456"}' 127.0.0.1:9090 config.ConfigService/AddBlacklist

grpcurl -plaintext -proto ./proto/config.proto -d '{"item": "123"}' 127.0.0.1:9090 config.ConfigService/RemoveBlacklist
```