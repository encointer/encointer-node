cat accounts_all.txt | while read a
do
  BAL=$(../target/release/encointer-client-notee -u wss://kusama.api.encointer.org -p 443 balance $a --cid u0qj92QX9PQ --at 0xcfb07c60aadd57676ce0591678b58511ebd03bdef7385c9690f42e744f1dbff6)
  # translate Kusama prefix to substrate prefix for easy search
  ACC=$(subkey inspect $a --network substrate | grep -P '^[\s]+SS58' | sed 's/^ *SS58 Address: *//')
  echo $a,$ACC,$BAL
done
