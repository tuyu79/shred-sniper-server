# 测试 whitelist grpc api
```shell
grpcurl -plaintext -proto ./proto/whitelist.proto -d '{
                                                        "profit": 0.1,
                                                        "avg": 10,
                                                        "count": 1,
                                                        "mid": 2,
                                                        "hold_less_5_sec_count": 10,
                                                        "min_hold": 5,
                                                        "avg_user": 10,
                                                        "top_3_buy": 1
                                                      }' 127.0.0.1:8090 whitelist.WhitelistService/GetWhitelist
```