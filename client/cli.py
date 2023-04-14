#!/usr/bin/env python3

# Helper script predominantly used to bring the node to a certain state while testing with the app.

import click

from py_client.client import Client
from py_client.scheduler import CeremonyPhase


@click.group()
@click.pass_context
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
def cli(ctx, client, url, port):
    ctx.obj['client'] = Client(
        rust_client=client,
        node_url=url,
        port=port
    )


@cli.command()
@click.option('--cid',
              default='sqm1v79dF6b',
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.pass_context
def register_alice_bob_charlie_and_go_to_attesting(ctx, cid: str):
    client = ctx.obj['client']

    _register_alice_bob_charlie(client, cid)

    print(client.go_to_phase(CeremonyPhase.Attesting))


@cli.command()
@click.option('--cid',
              default='sqm1v79dF6b',
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.pass_context
def register_alice_bob_charlie_and_go_to_assigning(ctx, cid: str):
    client = ctx.obj['client']

    _register_alice_bob_charlie(client, cid)

    print(client.go_to_phase(CeremonyPhase.Assigning))


@cli.command()
@click.option('--cid',
              default='sqm1v79dF6b',
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.pass_context
def register_alice_bob_charlie(ctx, cid: str):
    click.echo(f'Registering Alice, Bob and Charlie for cid: {cid}')

    client = ctx.obj['client']

    accounts = ['//Alice', '//Bob', '//Charlie']

    register(accounts, client, cid, should_faucet=False)


@cli.command()
@click.option('--cid',
              default='sqm1v79dF6b',
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.option('--should_faucet',
              default=False,
              help='If newbies should be fauceted')
@click.pass_context
def register_gina_harry_ian(ctx, cid: str, should_faucet: bool):
    """ Registers accounts who aren't bootstrappers in the mediterranean test currency """
    client = ctx.obj['client']

    click.echo(f'Registering Gina, Harry and Ian for cid: {cid}')
    click.echo(f'Fauceting the new accounts: {should_faucet}')

    # newbies in the mediterranean test-currency
    accounts = ['//Gina', '//Harry', '//Ian']

    register(accounts, client, cid, should_faucet)


def register(accounts, client: Client, cid: str, should_faucet=False):
    print(client.list_communities())
    print(client.go_to_phase(CeremonyPhase.Registering))

    if should_faucet:
        client.faucet(accounts, is_faucet=True)
        client.await_block()

    for acc in accounts:
        client.register_participant(acc, cid)

    print('Awaiting next block')
    client.await_block()


@cli.command()
@click.pass_context
def registering_phase(ctx):
    click.echo(f'Going to registering phase')
    client = ctx.obj['client']

    if CeremonyPhase[client.get_phase()] == CeremonyPhase.Registering:
        client.next_phase()

    print(client.go_to_phase(CeremonyPhase.Registering))


@cli.command()
@click.pass_context
def next_phase(ctx):
    click.echo(f'Going to next phase')
    client = ctx.obj['client']
    print(client.next_phase())


@cli.command()
@click.pass_context
def get_phase(ctx):
    click.echo(f'Get current phase')
    client = ctx.obj['client']
    print(client.get_phase())


if __name__ == '__main__':
    cli(obj={})
