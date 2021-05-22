import argparse

default_client = '../target/release/encointer-client-notee'


def simple_parser(add_help=False):
    """ Create a simple parser that adds [client] and [port] arguments.

        Help must be false if the parser is passed as a parent to another parser
        to prevent duplicate declaration. But the `simple_parser`s arguments
        will be shown regardless.
    """

    p = argparse.ArgumentParser(add_help=add_help)
    p.add_argument('--client',
                   default=default_client,
                   help=f'The rust client binary that should be used. (default={default_client})')
    p.add_argument('--port',
                   default=9944,
                   help='Port of the node (default=9944).')
    return p
