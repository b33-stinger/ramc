### Rust Arch Mirror Checker v1.0.3
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
--max-check <MAX_CHECK>                   Max Mirrors to check (-1 == all)        [default: -1]
-l, --log-file <LOG_FILE>                 log file name                           [default: log.json]
-c, --continous-log                       dump to log after each iteration        [default: false]
-e, --exclude-country <EXCLUDE_COUNTRY>   Exclude Country                         [default: None]
-i, --include-country <INCLUDE_COUNTRY>   Exclude Country                         [default: None]
-q, --quiet                               Don't log (stdout) data                 [default: false]
-n, --no-log                              Don't log (file) data                   [default: false]
-t, --timeout <TIMEOUT>                   Request Timeout                         [default: 30]
-u, --user-agent <USER_AGENT>             User Agent                              [default: None]
-s, --skip-ssl                            Disable SSL verification (For all)      [default: false]
-m, --max-threads <MAX_THREADS>           Maximum Threads to use (-1 == No Limit) [default: -1]
-h, --help                                Print help
```


#### Python Version
##### https://github.com/b33-stinger/amc
#### Comparison
```
time ./amc.py -t 10 -n 10     time ./ramc -t 10

real    0m51.352s             real    0m10.748s   -3.21× (79.10% faster) (Same as last Vesion)
user    0m2.207s              user    0m0.749s    -3.18× (66.10% faster) (Same as last Vesion)
sys     0m0.300s              sys     0m0.230s    -1.21× (23.30% faster) (Same as last Vesion)
```