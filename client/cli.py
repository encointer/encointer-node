#!/usr/bin/env python3

# Helper script predominantly used to bring the node to a certain state while testing with the app.

import click

from py_client.client import Client
from py_client.scheduler import CeremonyPhase

latam_cid = '3zz714jWojt'
latam1 = '//LATAM1'
latam2 = '//LATAM2'
latam3 = '//LATAM3'
latam_accounts = [latam1, latam2, latam3]


def _attest_latam_meetup(client, cid):
    print('Starting meetup...')
    client.attest_attendees(latam1, cid, [latam2, latam3])
    client.attest_attendees(latam2, cid, [latam1, latam3])
    client.attest_attendees(latam3, cid, [latam1, latam2])


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

    register_alice_bob_charlie(client, cid)

    print(client.go_to_phase(CeremonyPhase.Attesting))


@cli.command()
@click.option('--cid',
              default='sqm1v79dF6b',
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.pass_context
def register_alice_bob_charlie_and_go_to_assigning(ctx, cid: str):
    client = ctx.obj['client']

    register_alice_bob_charlie(client, cid)

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
              default=latam_cid,
              help='CommunityIdentifier. Default is Mediterranean test currency')
@click.pass_context
def register_latam_accounts(ctx, cid: str):
    # The demo should do this manually this is just for testing convenience
    click.echo(f'Registering Alice, Bob and Charlie for cid: {cid}')

    client = ctx.obj['client']

    accounts = ['//LATAM1', '//LATAM2', '//LATAM3']

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


@cli.command()
@click.option('--cid',
              default=latam_cid,
              help='CommunityIdentifier. Default is Mediterranean test currency')
def perform_latam_meetup_gsl(cid: str):
    click.echo(f'Performing meetup for //LATAM1, //LATAM2, //LATAM3 cid: {cid}')

    client = Client(
        node_url="wss://gesell.encointer.org",
        rust_client="../target/release/encointer-client-notee",
        port=443
    )

    _attest_latam_meetup(client, cid)

    print(f"Waiting for {1} block, such that xt's are processed...")
    client.await_block(1)

    # print(f"Listing Attestees")
    # print(client.list_attestees(cid))


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
