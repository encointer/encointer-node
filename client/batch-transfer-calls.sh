CID=u0qj944rhWE
SENDER=5GTUm6tgZwqn8pqinRz3tAKAXinXHUnncCWVs5mvYqfZtz4v
while IFS="," read -r accountk account amount
do
  #echo "$account $amount"
  CALL=$(encointer-cli -u wss://kusama.api.encointer.org -p 443 transfer $SENDER $account $amount --cid $CID --dryrun)
  echo "$CALL"
done < $1

