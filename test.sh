cd logos-derive
cargo test || exit
cd ..

cd tests
cargo test || exit
cd ..

cd logos
cargo test || exit
cargo test --no-default-features --features export_derive || exit
