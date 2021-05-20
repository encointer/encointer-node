import geojson

from math import sqrt, floor
from pyproj import Geod

geoid = Geod(ellps='WGS84')


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


def generate_community_spec(meta, locations, bootstrappers):
    print("Community metadata: " + str(meta))

    gj = geojson.FeatureCollection(list(map(lambda x: geojson.Feature(geometry=x), locations)))
    gj['community'] = {'meta': meta, 'bootstrappers': bootstrappers}
    fname = meta['name'] + '.json'
    with open(fname, 'w') as outfile:
        geojson.dump(gj, outfile, indent=2)
    return fname


def meta_json(name, symbol, icons_cid="Defau1tCidThat1s46Characters1nLength1111111111"):
    return {"name": name, "symbol": symbol, "icons": icons_cid}
