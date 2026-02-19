#!/bin/bash

PARAMS="${@:2}"
#printf "==>[entrypoint.sh] %s\n" "PARAMS=$PARAMS"

case $1 in

  encointer-cli)
    /encointer-cli $PARAMS
    ;;

  bootstrap_demo_community.py)
    /bootstrap_demo_community.py --client /encointer-cli $PARAMS
    ;;

  bot-community-test)
    # Example:
    # docker run -it --add-host host.docker.internal:host-gateway test-client bot-community-test -u ws://host.docker.internal --port 9944 -f http://host.docker.internal:5000/api

    /bot-community.py --client /encointer-cli $PARAMS init
    /bot-community.py --client /encointer-cli $PARAMS simulate --ceremonies 7
    diff bot-stats.csv bot-stats-golden.csv
    ;;

  phase.py)
    # Example:
    # docker run -it --add-host host.docker.internal:host-gateway test-client phase.py -u ws://host.docker.internal --port 9944 --idle-blocks 3
    /phase.py --client /encointer-cli $PARAMS
    ;;

  faucet.py)
    # Example: Note: we have to expose the port
    # docker run -it -p 5000:5000 --add-host host.docker.internal:host-gateway test-client faucet.py -u ws://host.docker.internal --port 9944
    /faucet.py --client /encointer-cli $PARAMS
    ;;

# Todo #386: Not working yet; bot-community, and register-random-businesses-and-offering have different interface.
#  test-register-businesses)
#    /bot-community.py --client /encointer-cli $PARAMS init
#    /register-random-businesses-and-offerings.py --client /encointer-cli $PARAMS
#    ;;

  *)
    echo -e 'Usage: docker run -it encointer/encointer-cli:<version> [encointer-cli|bootstrap_demo_community.py|cli.py] <params>'
    echo -e 'Example to talk to a node on the host machine:'
    echo -e 'docker run -it encointer/encointer-cli:<version> encointer-cli list-communities -u ws://host.docker.internal -p 9944'
    exit
    ;;
esac