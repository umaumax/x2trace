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

## trace.json view tool
自動的にファイルを読み込むことのできるちょうどよい方法がない(URLに読み込み先を指定して簡単にreloadなどができると理想)

* [chromium \- Load json manually in chrome://tracing \- Stack Overflow]( https://stackoverflow.com/questions/49147681/load-json-manually-in-chrome-tracing )
  * [jlfwong/speedscope: 🔬 A fast, interactive web\-based viewer for performance profiles\.]( https://github.com/jlfwong/speedscope#usage )
* [loading \- Programmatically open a json file in chrome://tracing, from a Chrome extension \- Stack Overflow]( https://stackoverflow.com/questions/42076654/programmatically-open-a-json-file-in-chrome-tracing-from-a-chrome-extension?noredirect=1&lq=1 )
