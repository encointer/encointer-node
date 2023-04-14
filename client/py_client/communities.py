import geojson

from math import sqrt
from pyproj import Geod
from random_words import RandomWords

from py_client.helpers import mkdir_p

geoid = Geod(ellps='WGS84')

COMMUNITY_SPECS_PATH = '.communities'


def move_point(point, az, dist):
    """ move a point a certain distance [meters] into a direction (azimuth) in [degrees] """

    lng_new, lat_new, return_az = geoid.fwd(point['coordinates'][0], point['coordinates'][1], az, dist)
    return geojson.Point([lng_new, lat_new])


def populate_locations(northwest, n, dist=1000):
    """ populate approximately n locations on a square grid of a specified distance in meters """
    row = [northwest]
    for li in range(1, round(sqrt(n))):
        row.append(move_point(row[-1], 90, dist))
    locations = []
    for pnt in row:
        col = [pnt]
        for li in range(1, round(sqrt(n))):
            col.append(move_point(col[-1], 180, dist))
        locations += col
    return locations


def random_community_spec(bootstrappers, ipfs_cid, locations_count):
    point = geojson.utils.generate_random("Point", boundingBox=[-56, 41, -21, 13])
    locations = populate_locations(point, locations_count)
    print(f'created {len(locations)} random locations around {point}.')

    name = 'bot-' + '-'.join(RandomWords().random_words(count=1))
    symbol = name[1:4].upper()
    meta = meta_json(name, symbol, ipfs_cid)
    print(f'CommunityMetadata {meta}')
    return generate_community_spec(meta, bootstrappers, locations)


def generate_community_spec(meta, bootstrappers, locations):
    print("Community metadata: " + str(meta))

    gj = geojson.FeatureCollection(list(map(lambda x: geojson.Feature(geometry=x), locations)))
    gj['community'] = {'meta': meta, 'bootstrappers': bootstrappers}
    fname = f"{COMMUNITY_SPECS_PATH}/{meta['name']}.json"

    mkdir_p(COMMUNITY_SPECS_PATH)

    with open(fname, 'w') as outfile:
        geojson.dump(gj, outfile, indent=2)
    return fname


def meta_json(name, symbol, assets_cid="Defau1tCidThat1s46Characters1nLength1111111111"):
    return {"name": name, "symbol": symbol, "assets": assets_cid}
