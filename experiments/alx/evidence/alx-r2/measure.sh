#!/bin/bash
R=/c/workspace/PragmaWorks/loom-archive; RUN=$R/alx-r2/$1; M=$RUN/measure
export PATH="/c/Users/juang/.cargo/bin:$PATH:$(ls -d "$LOCALAPPDATA"/Microsoft/WinGet/Packages/*WinLibs*/mingw64/bin | head -1)"
rm -rf $M; mkdir -p $M
(cd $RUN/derived && tar cf - --exclude=target --exclude=tests .) | (cd $M && tar xf -)
cp -r $R/alx-r/measure/tests $R/alx-r/measure/corpus $M/ 2>/dev/null; cp $R/alx-r/measure/loom.toml $M/ 2>/dev/null
python $R/alx-r2/patch_cli.py $M/tests/cli_test.rs
cd $M; P=0; T=0; : > $RUN/per-target.tsv
for f in tests/*_test.rs; do n=$(basename $f .rs)
  out=$(cargo test -j 2 --target-dir $RUN/derived/target --test $n 2>&1)
  # sum all 'test result' lines of the target binary (includes nested runs; take max-passed line = the harness itself)
  read p fl < <(echo "$out" | grep -E '^test result' | sed -E 's/.* ([0-9]+) passed; ([0-9]+) failed.*/\1 \2/' | sort -n | tail -1)
  t=$(echo "$out" | grep -E '^running [0-9]+ test' | sed -E 's/running ([0-9]+).*/\1/' | sort -n | tail -1)
  [ -z "$t" ] && t=$(grep -cE '#\[(tokio::)?test\]' $f) && p=0
  P=$((P+${p:-0})); T=$((T+t)); echo -e "$n\t${p:-0}\t$t" >> $RUN/per-target.tsv
done
echo "$1 passed=$P total=$T S=$(python -c "print(round($P/$T,3))")"
