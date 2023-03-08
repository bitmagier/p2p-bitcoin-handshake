# Sample P2P node handshake for bitcoin

## Test Procedure

We build and run a bitcoin-core node and run our tool against it.

Install Dependencies. See [dependencies.md](https://github.com/bitcoin/bitcoin/blob/master/doc/dependencies.md)

e.g. for Ubuntu jammy (22.04):

```bash
apt install build-essential autoconf automake clang libboost-all-dev
```

---

Compile original Bitcoin core version v24.0.1

```bash
git submodule update --init
cd bitcoin-core
# we checkout a stable recent version to get a consistent result 
git checkout v24.0.1
./autogen.sh
./configure --disable-maintainer-mode --disable-wallet --disable-tests --disable-bench --with-gui=no
make # use "-j N" for N parallel jobs
cd ..
```

---
Run Bitcoin Core node with a minimal chain and bind it to 127.0.0.1:

```bash
mkdir -p /tmp/bitcoin_data && bitcoin-core/src/bitcoind -datadir=/tmp/bitcoin_data -chain=regtest -bind=127.0.0.1 -debug=net
```

---
In another shell, we test the handshake implementation against the default port of regtest chain: 18445

```bash
cargo run -- --remote 127.0.0.1:18445 
```

# Resources

- [bitcoin node protocol documentation](https://en.bitcoin.it/wiki/Protocol_documentation)
- [protocol](https://www.oreilly.com/library/view/mastering-bitcoin/9781491902639/ch06.html)
- [original task](https://github.com/eqlabs/recruitment-exercises/blob/master/node-handshake.md)
