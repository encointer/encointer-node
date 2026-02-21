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
    # docker run -it --add-host host.docker.internal:host-gateway test-client bot-community-test -u ws://host.docker.internal --port 9944 -f http://host.docker.internal:7070/api

    /bot-community.py --client /encointer-cli $PARAMS init
    /bot-community.py --client /encointer-cli $PARAMS simulate --ceremonies 7
    diff bot-stats.csv bot-stats-golden.csv
    ;;

  ceremony-phase-and-faucet-service.py)
    # Example:
    # docker run -it -p 7070:7070 --add-host host.docker.internal:host-gateway test-client ceremony-phase-and-faucet-service.py -u ws://host.docker.internal --port 9944
    /ceremony-phase-and-faucet-service.py --client /encointer-cli $PARAMS
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