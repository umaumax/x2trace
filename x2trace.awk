#!/usr/bin/awk -f
BEGIN {
  ts_unit=1000*1000
  printf "[\n"
}

{flag=1}
/\+\+\+/ {flag=0}
/---/ {flag=0}
/<unfinished \.\.\.>/ {flag=0}
/<no return \.\.\.>/ {flag=0}
flag==1 {
  if (NR>1) {
    printf ",\n"
  }
  pid=$1
  tid=pid
  ts=$2*ts_unit
  dur=$NF
  gsub(/<|>/, "", dur)
  dur*=ts_unit
  name=$3
  # NOTE: <.... xxxx resumed>
  if (name=="<...") {
    name=$4
    ts-=dur
  }
  gsub(/\(.*/, "", name)
  printf "{\"name\":\"%s\",\"cat\":\"\",\"ph\":\"X\",\"ts\":%d,\"dur\":%d,\"pid\":%d,\"tid\":%d}", name, ts, dur, pid, tid
}

END {
  printf "\n]\n"
}
