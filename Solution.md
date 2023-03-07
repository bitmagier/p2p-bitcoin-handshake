# p2p candidates

- Bitcoin https://developer.bitcoin.org/devguide/p2p_network.html
    - Light Node  (Lightweight nodes are served by full nodes to connect to the Bitcoin network. They are effectively dependent on the full nodes to function.)

## build bitcoin node:

dependencies: https://github.com/bitcoin/bitcoin/blob/master/doc/dependencies.md

e.g. for Ubuntu jammy (22.04):  
```bash
apt install build-essential autoconf automake clang libboost-all-dev
```

Build real bitcoin node:
```bash
git submodule update --init
cd bitcoin
./autogen.sh
./configure --disable-maintainer-mode --disable-wallet --disable-tests --disable-bench --with-gui=no
make # use "-j N" for N parallel jobs
```

Run real bitcoin node:
```bash
mkdir -p /tmp/bitcoin_data && src/bitcoind -datadir=/tmp/bitcoin_data -prune=550 -testnet -bind=127.0.0.1:18334 -assumevalid=000000004e9044f75d5e40a6ee87a608bd06404c7ddcd5c609e9fba96a073ed8
```

Test handshake implementation:
```bash
cargo run -- --remote 127.0.0.1:18334 
```


# Resources
- [bitcoin node protocol documentation](https://en.bitcoin.it/wiki/Protocol_documentation)
- [protocol](https://www.oreilly.com/library/view/mastering-bitcoin/9781491902639/ch06.html)


# TODOS:
- visibility
- unit tests
