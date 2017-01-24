# Chatbix, the Chatbox with a typo

## Build

```sh
$ echo 'DATABASE\_URL=postgres://dbuser:dbpassword@dbaddress/chatbox' > .env
$ rustup override nightly
$ cargo run --bin chatbix
```

## License

Dual licensed under MIT / Apache-2.0
