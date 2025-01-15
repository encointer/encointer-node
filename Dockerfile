FROM ubuntu:22.04
LABEL description="This is the 2nd stage: a very small image where we copy the Substrate binary."

RUN apt-get update && \
apt-get install -y jq python3 python3-pip

RUN python3 -m pip install --upgrade pip
RUN pip install geojson pyproj RandomWords wonderwords requests flask substrate-interface click

RUN mv /usr/share/ca* /tmp && \
	rm -rf /usr/share/*  && \
	mv /tmp/ca-certificates /usr/share/ && \
	useradd -m -u 1000 -U -s /bin/sh -d /encointer encointer && \
	mkdir -p /encointer/.local/share/encointer-collator && \
	chown -R encointer:encointer /encointer/.local && \
	ln -s /encointer/.local/share/encointer-collator /data

WORKDIR /

COPY scripts/docker/entryscript.sh /
COPY encointer-client-notee /

#COPY ./scripts/healthcheck9933.sh /usr/local/bin

RUN mkdir /client
COPY client/py_client /py_client
COPY client/test-data /test-data

# all python scripts (some of them aren supported by the entryfile.sh yet).
COPY client/bootstrap_demo_community.py /
COPY client/bot-community.py /
COPY client/bot-stats-golden.csv /
COPY client/cli.py /
COPY client/faucet.py /
COPY client/phase.py /
COPY client/typedefs.json /
COPY client/register-random-businesses-and-offerings.py /

RUN chmod +x /encointer-client-notee
#RUN chmod +x /usr/local/bin/healthcheck9933.sh

# checks
RUN ldd /encointer-client-notee && \
	/encointer-client-notee --version

# Shrinking
#RUN rm -rf /usr/lib/python* && \
#	rm -rf /usr/bin /usr/sbin /usr/share/man

#USER encointer
EXPOSE 30333 9933 9944 9615 5000
VOLUME ["/data"]

ENTRYPOINT ["/entryscript.sh"]
