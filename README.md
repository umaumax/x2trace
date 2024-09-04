# x2trace

ltrace,straceなどのトレーサーの出力結果を`trace.json`形式に変換する

マルチスレッドで各スレッドがどのような関数を呼び出しているのかをわかりやすく可視化したいという目的

* [x] `ltrace`: by awk tool
* [x] `strace`: by awk tool
* [x] `iftracer`: by rust tool

## for [umaumax/iftracer]( https://github.com/umaumax/iftracer/tree/master/ )
``` bash
cargo run --release -- iftracer iftracer.out.XXX --bin $BIN_FILEPATH

# for another arch
OBJDUMP=/usr/bin/arm-linux-gnueabihf-objdump cargo run --release -- iftracer iftracer.out.XXX --bin $BIN_FILEPATH
```

ASLR(address space layout randomization)を無効化して、iftracerの実行ファイルの実行方法
``` bash
setarch $(uname -m) -R ./a.out
```

`/proc/$PID/maps`
``` log
00400000-004c1000 r-xp 00000000 08:02 9044616                            /home/user/hoge_command
006c0000-006c1000 r--p 000c0000 08:02 9044616                            /home/user/hoge_command
006c1000-006c8000 rw-p 000c1000 08:02 9044616                            /home/user/hoge_command
006c8000-006db000 rw-p 00000000 00:00 0 
00f18000-01eb1000 rw-p 00000000 00:00 0                                  [heap]
7f27cf08a000-7f27cf24a000 r-xp 00000000 08:02 42507243                   /home/user/libhoge.so
7f27cf24a000-7f27cf44a000 ---p 001c0000 08:02 42507243                   /home/user/libhoge.so
7f27cf44a000-7f27cf44e000 r--p 001c0000 08:02 42507243                   /home/user/libhoge.so
7f27cf44e000-7f27cf450000 rw-p 001c4000 08:02 42507243                   /home/user/libhoge.so
```

`--bin`に`libhoge.so`を指定するときには、`--proc-maps=/pro/$PID/maps`か`--base-address=7f27cf08a000`とすることで実行時にアドレスが決定される共有ライブラリの名前解決ができる

## トレース結果の検証/加工ツール
[x2trace/tools]( https://github.com/umaumax/x2trace/tree/master/tools )

* 外れ値検出
* フィルタリングツール

## how to use
open `chrome://tracing` and drop output file

## memo
* rustの公式ツールを見ると，`trace.json`の出力に`serde_json`を利用している
  * [measureme/main\.rs at master · rust\-lang/measureme]( https://github.com/rust-lang/measureme/blob/master/crox/src/main.rs )
* [パフォーマンス計測に困らない！tracing活用術100 \- Qiita]( https://qiita.com/keishi/items/5f1af0851901e9021488 )
  * `trace.json`のGUIツールの使い方

----

## ./x2trace.awk
### ltrace
```
seq 1 10 | ltrace -ttt -T -f -o ltrace-ttt-T-f-o.xargs.log xargs -L1 -I{} -P 4 bash -c "sleep 1 & echo {}"
cat ltrace-ttt-T-f-o.xargs.log | ./x2trace.awk > xargs.json

# with system call
seq 1 10 | ltrace -S -ttt -T -f -o ltrace-S-ttt-T-f-o.xargs.log xargs -L1 -I{} -P 4 bash -c "sleep 1 & echo {}"
cat ltrace-S-ttt-T-f-o.xargs.log | ./x2trace.awk > xargs.json
```

* 共有ライブラリの同一の関数内で再帰呼び出しがないことが前提の処理
  * ltraceでindent付きで記録している場合はそのindentから区別することは可能
* ltraceのtsはおそらく，呼び出されたときの時間であると思われる
  * resumedのts - durがunfinishedのtsと一致するわけではない
* `-e '*'`とするとライブラリファイル名付きで出力され，関数名が不明のものはアドレスで示される
* 標準出力とファイル出力(`-o`)で出力formatが微妙に異なる
* ltraceの影響で上記のプログラムは相当処理が遅くなっている

### strace
```
seq 1 10 | strace -ttt -T -f -q -o strace-ttt-T-f-q-o.xargs.log xargs -L1 -I{} -P 4 bash -c "sleep 1 & echo {}"
cat strace-ttt-T-f-q-o.xargs.log | ./x2trace.awk > xargs.json
```

## bpftrace-sched-switch.py
``` bash
./bpftrace-sched-switch.py --example | gzip -c > trace.json.gz
./bpftrace-sched-switch.py --example --pid-comm-cmdline ./data/pid-comm-cmdline.csv | gzip -c > trace.json.gz
./bpftrace-sched-switch.py --pid-comm-cmdline ./data/pid-comm-cmdline.csv ./data/bpftrace-sched-switch.jsonl | gzip -c > trace.json.gz
```

## perf-sched-timehist.py
* 一番上の`CPU: XXX`をexpandすると各CPUごとのプロセスの割当がわかる
  * expandしない場合にはヒストグラムでCPU使用率がわかる

timehist形式
``` bash
sudo perf sched record sleep 1

sudo perf sched timehist > timehist.log

# overwrite CLOCK_MONOTONIC timestamp to CLOCK_REALTIME timestamp
# e.g. 1646610039.304117326
sudo perf sched timehist | perl -pe 'BEGIN{$offset=shift} s/^ *([0-9]+.[0-9]+)/$1+$offset/e' $TIMESTAMP_OFFSET > timehist.log

./perf-sched-timehist.py timehist.log -o timehist-trace.json

./perf-sched-timehist.py data/perf-sched-timehist.txt -o perf-timehist-trace.json
```

通常形式
``` bash
sudo perf record -T -a -e sched:sched_switch -e 'irq:*' -- sleep 1
sudo perf script > perf.data.log

./perf-sched-timehist.py perf.data.log -o perf.data-trace.json
```

* how to get realtime - monotonic time offset
  * c++: [get timestamp offset(realtime - monotonic)]( https://gist.github.com/umaumax/587238da2b1adad9c85f600076b7280e )
  * rust: [umaumax/tsd-rs]( https://github.com/umaumax/tsd-rs )

### NOTE
* CPU名を明示的にするとsliceのデータの発生順の表示となる
* chrome tracingではjson形式の他にzip形式もドラッグ・アンド・ドロップ可能であるので、`x2trace`でzip形式にする機能をつけるとよいかもしれない
  * `perf-sched-timehist.py`にもzip出力の機能があると良いのかもしれない

## trace.json view tool
自動的にファイルを読み込むことのできるちょうどよい方法がない(URLに読み込み先を指定して簡単にreloadなどができると理想)

* [chromium \- Load json manually in chrome://tracing \- Stack Overflow]( https://stackoverflow.com/questions/49147681/load-json-manually-in-chrome-tracing )
  * [jlfwong/speedscope: 🔬 A fast, interactive web\-based viewer for performance profiles\.]( https://github.com/jlfwong/speedscope#usage )
* [loading \- Programmatically open a json file in chrome://tracing, from a Chrome extension \- Stack Overflow]( https://stackoverflow.com/questions/42076654/programmatically-open-a-json-file-in-chrome-tracing-from-a-chrome-extension?noredirect=1&lq=1 )

## perfetto
引数で指定したファイルをネットワーク経由で取得してperfetto上に表示する
``` bash
pip install "git+https://github.com/umaumax/x2trace.git#subdirectory=perfetto"
perfetto-server $TARGET_TRACE_FILEPATH
```

``` bash
cd ./perfetto
pip install . # for users
pip install -e . # for developers
# or
cd ./perfetto_server
./main.py -p 60080 ../../trace.json.gz
```
