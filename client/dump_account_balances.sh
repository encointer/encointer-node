cat accounts_all.txt | while read a
do
  BAL=$(../target/release/encointer-client-notee -u wss://kusama.api.encointer.org -p 443 balance $a --cid u0qj944rhWE --at 0xe3310c3d23bb95a618f72cca98a512c1e928923e80997bd29d121bc66bcb8a86)
  # translate Kusama prefix to substrate prefix for easy search
  ACC=$(subkey inspect $a --network substrate | grep -P '^[\s]+SS58' | sed 's/^ *SS58 Address: *//')
  echo $a,$ACC,$BAL
done
