cd tests
cargo test || exit

cd ..
cd logos-derive
cargo test || exit
cd ..

cd logos
cargo test || exit
