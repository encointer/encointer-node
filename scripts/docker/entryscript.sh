#!/bin/bash

PARAMS="${@:2}"
#printf "==>[entrypoint.sh] %s\n" "PARAMS=$PARAMS"

case $1 in

  encointer-client-notee)
    /encointer-client-notee $PARAMS
    ;;

  bootstrap_demo_community.py)
    /bootstrap_demo_community.py --client /encointer-client-notee $PARAMS
    ;;

  bot-community-test)
    /bot-community.py --client /encointer-client-notee $PARAMS init
#    /bot-community.py --client /encointer-client-notee $PARAMS test
#    diff bot-stats.csv bot-stats-golden.csv
    ;;

  phase.py)
    /phase.py --client /encointer-client-notee $PARAMS
    ;;

  faucet.py)
    /faucet.py --client /encointer-client-notee $PARAMS
    ;;

  test-register-businesses)
    /bot-community.py --client /encointer-client-notee $PARAMS init
    /register-random-businesses-and-offerings.py --client /encointer-client-notee $PARAMS
    ;;

# Does not work yet because the script wants the options like: cli.py --client <client> -u url -p port <cmd> <command params>
#  cli.py)
#    /cli.py $PARAMS
#    ;;

  *)
    echo -e 'Usage: docker run -it encointer/encointer-client-notee:<version> [encointer-client-notee|bootstrap_demo_community.py|cli.py] <params>'
    echo -e 'Example to talk to a node on the host machine:'
    echo -e 'docker run -it encointer/encointer-client-notee:<version> encointer-client-notee list-communities -u ws://host.docker.internal -p 9944'
    exit
    ;;
esac