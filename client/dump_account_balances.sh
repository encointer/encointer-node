cat accounts_all.txt | while read a
do
  BAL=$(../target/release/encointer-client-notee -u wss://kusama.api.encointer.org -p 443 balance $a --cid u0qj944rhWE --at 0xa6f22967a9b642ce67bfb473889c899be918e94fee35794227617260a9eea811)
  # translate Kusama prefix to substrate prefix for easy search
  ACC=$(subkey inspect $a --network substrate | grep -P '^[\s]+SS58' | sed 's/^ *SS58 Address: *//')
  echo $a,$ACC,$BAL
done
