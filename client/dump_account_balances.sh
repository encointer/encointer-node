cat accounts_all.txt | while read a
do
  BAL=$(../target/release/encointer-client-notee -u wss://kusama.api.encointer.org -p 443 balance $a --cid u0qj944rhWE --at 0x64e356817cefc0aa9353cac6d6e28169b9887b1e5df4535c218a7b553513e875)
  # translate Kusama prefix to substrate prefix for easy search
  ACC=$(subkey inspect $a --network substrate | grep -P '^[\s]+SS58' | sed 's/^ *SS58 Address: *//')
  echo $a,$ACC,$BAL
done
