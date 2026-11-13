import re,sys
p=sys.argv[1]; s=open(p).read()
s=re.sub(r'fn loom_bin\(\) -> PathBuf \{.*?\n\}\n','fn loom_bin() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_loom")) }\n',s,count=1,flags=re.S)
open(p,'w').write(s)
