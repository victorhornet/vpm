install:
    cargo install --path .

test:
    cargo watch -q -c -x "test -q -- --nocapture --color always"
