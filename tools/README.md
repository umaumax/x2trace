# tools

## how to setup
for outlier.py
``` bash
pip3 install rich
pip3 install numpy
```

## how to run
### outlier.py
``` bash
$ ./outlier.py chrome-tracing.json

$ ./outlier.py -h
usage: outlier.py [-h] [--min MIN] [--th TH] [--call CALL]
                  input [args [args ...]]

positional arguments:
  input
  args

optional arguments:
  -h, --help   show this help message and exit
  --min MIN    min duration of outlier[ms] (default: 100)
  --th TH      threshold of outlier (default: 100.0)
  --call CALL  min number of calls (default: 100)
```

### filter.py
``` bash
$ ./filter.py -b 2000 -e 3000 chrome-tracing.json

$ ./filter.py --in '.*hoge.*lambda.*' chrome-tracing.json

$ ./filter.py -h
usage: filter.py [-h] [-b BEGIN_TIMESTAMP] [-e END_TIMESTAMP]
                 [--include INCLUDE] [--exclude EXCLUDE]
                 input [args [args ...]]

positional arguments:
  input
  args

optional arguments:
  -h, --help            show this help message and exit
  -b BEGIN_TIMESTAMP, --begin-timestamp BEGIN_TIMESTAMP
                        begin timestamp[ms] (default: 0)
  -e END_TIMESTAMP, --end-timestamp END_TIMESTAMP
                        end timestamp[ms] (default: 3600000)
  --include INCLUDE, --in INCLUDE
                        include function name regex pattern (default: ^)
  --exclude EXCLUDE, --ex EXCLUDE
                        exclude function name regex pattern (default: $^)
```
