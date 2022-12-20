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

  cli.py)
    /cli.py $PARAMS
    ;;

  *)
    echo -e 'Usage: docker run -it encointer/encointer-client-notee:<version> [encointer-client-notee|bootstrap_demo_community.py|cli.py] <params>'
    echo -e 'Example:'
    echo -e 'docker run -it encointer/encointer-client-notee:<version> encointer-client-notee --version'
    exit
    ;;
esac