import argparse
import os

try:
    DEFAULT_CLIENT = os.environ['ENCOINTER_CLIENT']
except:
    print("didn't find ENCOINTER_CLIENT in env variables, setting client to ../target/release/encointer-client-notee")
    DEFAULT_CLIENT = '../target/release/encointer-client-notee'

def simple_parser(add_help=False):
    """ Create a simple parser that adds [client] and [port] arguments.

        Help must be false if the parser is passed as a parent to another parser
        to prevent duplicate declaration. But the `simple_parser`s arguments
        will be shown regardless.
    """

    p = argparse.ArgumentParser(add_help=add_help)
    p.add_argument('--client',
                   default=DEFAULT_CLIENT,
                   help=f'The rust client binary that should be used. (default={DEFAULT_CLIENT})')
    p.add_argument('--port',
                   default=9944,
                   help='Port of the node (default=9944).')
    p.add_argument('--node_url',
                   default=None,
                   help='url of remote chain')
    return p
