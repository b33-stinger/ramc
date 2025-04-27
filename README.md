### Rust Arch Mirror Checker v0.0.1
#### Check Arch ISO mirrors

#### Setup
##### Source
```bash
git clone https://github.com/b33-stinger/ramc.git
cd ramc
cargo build --release
```

### Start
```
mv target/release/ramc ./
./ramc --help
```


### Usage
```
-d, --download-url <DOWNLOAD_URL>         URL to get the Mirrors from             [default: https://archlinux.org/download/]
-a, --ask-custom-url                      ask for a custom download URL (stdin)   [default: false]
-m, --max-check <MAX_CHECK>               Max Mirrors to check (-1 == all)        [default: -1]
-l, --log-file <LOG_FILE>                 log file name                           [default: log.json]
-c, --continous-log                       dump to log after each iteration        [default: false]
-e, --exclude-country <EXCLUDE_COUNTRY>   Exclude Country                         [default: None]
-i, --include-country <INCLUDE_COUNTRY>   Exclude Country                         [default: None]
-q, --quiet                               Don't log (stdout) data                 [default: false]
-n, --no-log                              Don't log (file) data                   [default: false]
-h, --help                                Print help
```


#### Python Version
##### https://github.com/b33-stinger/amc
#### Comparison
```
time ./amc.py -t 10 -n 10     time ./ramc

real    0m51.352s             real    0m10.748s   -3.21× (79.10% faster) +32.5% (compared to 0.0.1)
user    0m2.207s              user    0m0.749s    -3.18× (66.10% faster) -7.7%  (compared to 0.0.1)
sys     0m0.300s              sys     0m0.230s    -1.21× (23.30% faster) +6.9%  (compared to 0.0.1)
```