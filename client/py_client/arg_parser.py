import argparse

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
    p.add_argument('--ipfs-api-key', dest='ipfs_api_key',
                   help=f'required api key to store files on remote ipfs node')
    return p
