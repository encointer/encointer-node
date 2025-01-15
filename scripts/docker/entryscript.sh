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
    # Example: note the port mapping, because we run the faucet at port 5000 in another container ...
    # docker run -it -p 5005:5000 --add-host host.docker.internal:host-gateway test-client bot-community-test -r ws://host.docker.internal --port 9944 -f http://host.docker.internal:5000/api

    /bot-community.py --client /encointer-client-notee $PARAMS init
    /bot-community.py --client /encointer-client-notee $PARAMS test
    diff bot-stats.csv bot-stats-golden.csv
    ;;

  phase.py)
    # Example: note the port mapping, because we run another the faucet exposing 5000
    # docker run -it -p 5001:5000 --add-host host.docker.internal:host-gateway test-client phase.py -r ws://host.docker.internal --port 9944 --idle-blocks 3
    /phase.py --client /encointer-client-notee $PARAMS
    ;;

  faucet.py)
    # Example:
    # docker run -it -p 5000:5000 --add-host host.docker.internal:host-gateway test-client faucet.py -u ws://host.docker.internal --port 9944
    /faucet.py --client /encointer-client-notee $PARAMS
    ;;

# not working yet, bot-commynity, and egister-random-businesses-and-offering have different interface. It is a pain.
#  test-register-businesses)
#    /bot-community.py --client /encointer-client-notee $PARAMS init
#    /register-random-businesses-and-offerings.py --client /encointer-client-notee $PARAMS
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