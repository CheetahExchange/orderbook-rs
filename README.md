# orderbook-rs

--------

### What is orderbook-rs

**orderbook-rs is a High Performance Order Matching Engine powered by Tokio.**

![](https://github.com/CheetahExchange/orderbook-rs/blob/main/asset/png/simple_architecture.png)


### Installing dependencies

* Install [Rust Compiler](https://www.rust-lang.org/learn/get-started) and Cpp linker

```
sudo apt-get update
sudo apt-get install git curl build-essential

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

* Install Redis-server and [Kafka](https://kafka.apache.org/)

```
sudo apt-get install redis-server
```

### Build orderbook-rs
```
git clone https://github.com/CheetahExchange/orderbook-rs
cd orderbook-rs

cargo clean
cargo build --release
```

### Deploy and Run

* update config.json with the correct parameters.

```
cp config_example.json config.json 
vi config.json
./orderbook-rs
```

* the running log looks like this

```
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:27 | trade_seq:7 | taker_order_id:33 | maker_order_id:28 | price:1.00 | size:998.00     
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:28 | order_id:28 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:29 | trade_seq:8 | taker_order_id:33 | maker_order_id:29 | price:2.00 | size:1.00       
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:30 | order_id:29 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:31 | trade_seq:9 | taker_order_id:33 | maker_order_id:30 | price:3.00 | size:1.00       
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:32 | order_id:30 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:33 | trade_seq:10 | taker_order_id:33 | maker_order_id:31 | price:4.00 | size:1.00      
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:34 | order_id:31 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:35 | trade_seq:11 | taker_order_id:33 | maker_order_id:32 | price:10.00 | size:499.00   
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:36 | order_id:33 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:37 | order:{"order_id":34,"user_id":1,"size":"1500.00","funds":"0.00","price":"1.00","side
":"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:38 | order:{"order_id":35,"user_id":1,"size":"1500.00","funds":"0.00","price":"1.00","side
":"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:39 | order:{"order_id":36,"user_id":1,"size":"2000.00","funds":"0.00","price":"10.00","sid
e":"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:40 | order:{"order_id":37,"user_id":1,"size":"1.00","funds":"0.00","price":"11.00","side":
"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:41 | order:{"order_id":38,"user_id":1,"size":"1.00","funds":"0.00","price":"12.00","side":
"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:42 | order:{"order_id":39,"user_id":1,"size":"1.00","funds":"0.00","price":"13.00","side":
"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\log.rs:82] new_open_log: product_id: BTC-USD | log_seq:43 | order:{"order_id":40,"user_id":1,"size":"3000.00","funds":"0.00","price":"1.00","side
":"sell","type":"limit","time_in_force":"GTC"}
[2023-09-27 17:08:17][src\matching\order_book.rs:239] Custom Error: existed val 40, order_id: 40
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:44 | trade_seq:12 | taker_order_id:41 | maker_order_id:34 | price:1.00 | size:1500.00   
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:45 | order_id:34 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:46 | trade_seq:13 | taker_order_id:41 | maker_order_id:35 | price:1.00 | size:1500.00   
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:47 | order_id:35 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:48 | order_id:41 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:195] new_match_log: product_id: BTC-USD | log_seq:49 | trade_seq:14 | taker_order_id:43 | maker_order_id:40 | price:1.00 | size:3000.00   
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:50 | order_id:40 | reason:DoneReasonFilled
[2023-09-27 17:08:17][src\matching\log.rs:135] new_done_log: product_id: BTC-USD | log_seq:51 | order_id:43 | reason:DoneReasonFilled

```

### How to Test

* place order test

```python
#!/usr/bin/env python
# encoding: utf-8

import logging
from kafka import KafkaProducer
from decimal import Decimal
import json

log_level = logging.DEBUG
logging.basicConfig(level=log_level)
log = logging.getLogger('kafka')
log.setLevel(log_level)


class Order:
    def __init__(self, _id, created_at, product_id, user_id, client_oid, price, size, funds, _type, side, time_in_force,
                 status):
        self.id = _id
        self.created_at = created_at
        self.product_id = product_id
        self.user_id = user_id
        self.client_oid = client_oid
        self.price = price
        self.size = size
        self.funds = funds
        self.type = _type
        self.side = side
        self.time_in_force = time_in_force
        self.status = status

producer = KafkaProducer(bootstrap_servers='127.0.0.1:9092')

order = Order(_id=43, created_at=1695783003020967000, product_id="BTC-USD", user_id=1, client_oid="",
              price=Decimal("20.00"), size=Decimal("3000.00"), funds=Decimal("0.00"), _type="limit",
              side="buy", time_in_force="GTC", status="new")

message = json.dumps(vars(order), default=str)
print(message)

producer.send('matching_order_BTC-USD', message.encode("utf8"))
producer.flush()
producer.close()
```



