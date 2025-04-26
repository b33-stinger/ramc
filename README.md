### Rust Arch Mirror Checker v0.0.1
#### Check Arch ISO mirrors

#### Setup
##### Source
```bash
git clone https://github.com/b33-stinger/ramc.git
cd ramc
cargo build --release
target/release/./ramc
# OR #
cargo run -- --help
```

### Start
`./[file]`


### Usage
```
  -d, --download-url <DOWNLOAD_URL>  URL to get the Mirrors from        [default: https://archlinux.org/download/]
  -l, --log-file <LOG_FILE>          log file name                      [default: log.json]
  -c, --continous-log                dump to log after each iteration   [default: false]
  -h, --help                         Print help
```


#### Python Version
##### https://github.com/b33-stinger/amc
#### Comparison
```
time ./amc.py -t 10 -n 10     time ./ramc

real    0m51.352s             real    0m15.982s   -3.21× (68.93% faster)
user    0m2.207s              user    0m0.695s    -3.18× (68.51% faster)
sys     0m0.300s              sys     0m0.247s    -1.21× (17.67% faster)
```