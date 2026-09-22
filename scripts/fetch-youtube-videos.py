#!/usr/bin/env python3
"""Refresh the list of channel videos rendered on /about/video-talks/.

Usage: python3 scripts/fetch-youtube-videos.py [--channel-id ID] [--output PATH]
       python3 scripts/fetch-youtube-videos.py --self-test

Reads YouTube's public per-channel Atom feed and writes
``hugo-site/data/youtube_videos.json``, which the ``youtube-video-grid``
shortcode renders at build time.

CI runs this before every Hugo build, so what gets deployed is whatever the
feed said moments earlier. It does not commit the result back: the copy in git
is a hand-refreshed snapshot, used only as the fallback when this script
fails, so a YouTube outage degrades to a stale list rather than a broken
deploy. Expect the committed copy to lag; that is not a sign the refresh is
broken.

The feed needs no API key and has no quota, which is why it is used in
preference to the YouTube Data API. Two limits come with that:

* it carries only the most recent 15 uploads, and
* it has no duration field, so Shorts cannot be told apart from talks.

Both are fine for a page that exists to show recent talks. If either becomes a
problem, the replacement is playlistItems.list from the Data API v3 (1 quota
unit per call against 10,000/day) writing this same JSON file, leaving the
shortcode untouched.

The script never overwrites the existing file with an empty or partial list:
any failure exits non-zero leaving the previous contents in place.

--self-test covers the parsing and writing above, offline. It deliberately
does not cover fetch_feed(), which would need a stub HTTP server to say
anything useful; that path is exercised for real on every CI build.
"""

import argparse
import datetime
import json
import os
import sys
import tempfile
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

# youtube.com/@FreenetOrg. The feed is addressed by channel id, not handle.
DEFAULT_CHANNEL_ID = "UCNVlXH0XHFqKoGL3RrbTy_w"
FEED_URL = "https://www.youtube.com/feeds/videos.xml?channel_id=%s"
DEFAULT_OUTPUT = os.path.join("hugo-site", "data", "youtube_videos.json")

NS = {
    "atom": "http://www.w3.org/2005/Atom",
    "yt": "http://www.youtube.com/xml/schemas/2015",
}

# Identify the build to YouTube rather than sending urllib's default, which is
# a plausible thing for them to rate-limit.
USER_AGENT = "freenet.org-site-build/1.0 (+https://freenet.org)"

FETCH_TIMEOUT_SECONDS = 30
FETCH_ATTEMPTS = 2

# The real feed is about 24 KB. This is a sanity bound, not a tuning knob: it
# exists so a response that is not really the feed cannot be read into memory
# without limit.
MAX_FEED_BYTES = 4 * 1024 * 1024


def parse_timestamp(value):
    """Return an aware datetime for an RFC 3339 string, or None if unusable.

    Python 3.10's fromisoformat does not accept a trailing 'Z', which the feed
    does not currently use but is valid RFC 3339, so normalise it first.
    """
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=datetime.timezone.utc)
    return parsed


def parse_feed(xml_bytes):
    """Return [{id, title, published, published_display}, ...], newest first.

    Entries are dropped unless they have a video id, a title, and a timestamp
    that actually parses. Dropping an unparseable date matters more than it
    looks: the template renders published_display verbatim, so a date this
    function let through unchecked would have had to be parsed by Hugo, and a
    date Hugo cannot parse fails the entire site build, not just this page.
    Formatting it here means no value from the feed can reach a function that
    is able to fail.

    Raises ElementTree.ParseError if the document is not XML at all.
    """
    root = ET.fromstring(xml_bytes)
    videos = []
    seen = set()
    for entry in root.findall("atom:entry", NS):
        video_id = (entry.findtext("yt:videoId", default="", namespaces=NS) or "").strip()
        title = (entry.findtext("atom:title", default="", namespaces=NS) or "").strip()
        published = (entry.findtext("atom:published", default="", namespaces=NS) or "").strip()
        if not video_id or not title or not published:
            continue
        timestamp = parse_timestamp(published)
        if timestamp is None:
            print("dropping %s: unparseable published value %r" % (video_id, published),
                  file=sys.stderr)
            continue
        if video_id in seen:
            continue
        seen.add(video_id)
        videos.append({
            "id": video_id,
            "title": title,
            "published": published,
            "published_display": "%s %d, %d" % (
                timestamp.strftime("%B"), timestamp.day, timestamp.year),
            "_sort_key": timestamp,
        })

    # The feed arrives newest-first already; sorting makes that a property of
    # this script rather than an assumption about YouTube. Sorting on the
    # parsed value rather than the string keeps it correct if entries ever
    # carry different UTC offsets.
    videos.sort(key=lambda v: v["_sort_key"], reverse=True)
    for video in videos:
        del video["_sort_key"]
    return videos


def fetch_feed(channel_id):
    """Return the raw feed bytes, retrying once on a transient failure."""
    url = FEED_URL % channel_id
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    last_error = None
    for attempt in range(FETCH_ATTEMPTS):
        try:
            with urllib.request.urlopen(request, timeout=FETCH_TIMEOUT_SECONDS) as response:
                body = response.read(MAX_FEED_BYTES + 1)
                if len(body) > MAX_FEED_BYTES:
                    raise SystemExit(
                        "%s returned more than %d bytes; refusing to parse it"
                        % (url, MAX_FEED_BYTES)
                    )
                return body
        except (urllib.error.URLError, OSError) as error:  # includes timeouts
            last_error = error
            print(
                "attempt %d/%d to fetch %s failed: %s"
                % (attempt + 1, FETCH_ATTEMPTS, url, error),
                file=sys.stderr,
            )
    raise SystemExit("could not fetch %s: %s" % (url, last_error))


def write_output(path, channel_id, videos):
    """Write the data file atomically so a crash cannot truncate it."""
    payload = {
        "channel_id": channel_id,
        "channel_url": "https://www.youtube.com/channel/%s" % channel_id,
        "videos": videos,
    }
    directory = os.path.dirname(path) or "."
    os.makedirs(directory, exist_ok=True)
    handle, temp_path = tempfile.mkstemp(dir=directory, suffix=".tmp")
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as out:
            # mkstemp creates 0600. Leaving it there would hand the build a
            # data file only the user that fetched it can read. Done on the
            # open descriptor so a failure here still closes it.
            os.fchmod(out.fileno(), 0o644)
            json.dump(payload, out, indent=2, ensure_ascii=False)
            out.write("\n")
        os.replace(temp_path, path)
    except BaseException:
        if os.path.exists(temp_path):
            os.unlink(temp_path)
        raise


SELF_TEST_FEED = """<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015"
      xmlns:media="http://search.yahoo.com/mrss/"
      xmlns="http://www.w3.org/2005/Atom">
 <title>Freenet</title>
 <entry>
  <id>yt:video:AAAAAAAAAAA</id>
  <yt:videoId>AAAAAAAAAAA</yt:videoId>
  <title>Older talk &amp; friends</title>
  <published>2026-09-19T22:07:04+00:00</published>
 </entry>
 <entry>
  <id>yt:video:BBBBBBBBBBB</id>
  <yt:videoId>BBBBBBBBBBB</yt:videoId>
  <title>Newest talk</title>
  <published>2026-09-21T01:19:22+00:00</published>
 </entry>
 <entry>
  <id>yt:video:CCCCCCCCCCC</id>
  <yt:videoId></yt:videoId>
  <title>Entry with no video id</title>
  <published>2026-09-20T00:00:00+00:00</published>
 </entry>
 <entry>
  <id>yt:video:DDDDDDDDDDD</id>
  <yt:videoId>DDDDDDDDDDD</yt:videoId>
  <title>Entry whose date cannot be parsed</title>
  <published>sometime last Tuesday</published>
 </entry>
 <entry>
  <id>yt:video:EEEEEEEEEEE</id>
  <yt:videoId>EEEEEEEEEEE</yt:videoId>
  <title>Entry timestamped with a trailing Z</title>
  <published>2026-09-20T12:00:00Z</published>
 </entry>
 <entry>
  <id>yt:video:BBBBBBBBBBB</id>
  <yt:videoId>BBBBBBBBBBB</yt:videoId>
  <title>Duplicate of the newest talk</title>
  <published>2026-09-21T01:19:22+00:00</published>
 </entry>
</feed>
"""


def self_test():
    """Check the parser offline, including the cases that must not render."""
    failures = []

    videos = parse_feed(SELF_TEST_FEED.encode("utf-8"))

    # Ordering, and the four entries that must not survive: no id, duplicate
    # id, and an unparseable date. The last one is the one that matters most:
    # letting it through would fail the whole site build, not just this page.
    expected_ids = ["BBBBBBBBBBB", "EEEEEEEEEEE", "AAAAAAAAAAA"]
    actual_ids = [v["id"] for v in videos]
    if actual_ids != expected_ids:
        failures.append("expected ids %r newest-first, got %r" % (expected_ids, actual_ids))
    if any(v["id"] == "DDDDDDDDDDD" for v in videos):
        failures.append("an entry with an unparseable published value was not dropped")

    by_id = {v["id"]: v for v in videos}
    if "AAAAAAAAAAA" in by_id and by_id["AAAAAAAAAAA"]["title"] != "Older talk & friends":
        failures.append("XML entities should be decoded, got %r" % by_id["AAAAAAAAAAA"]["title"])
    if "BBBBBBBBBBB" in by_id:
        newest = by_id["BBBBBBBBBBB"]
        if newest["published"] != "2026-09-21T01:19:22+00:00":
            failures.append("published timestamp not preserved, got %r" % newest["published"])
        if newest["published_display"] != "September 21, 2026":
            failures.append(
                "published_display should be the rendered date, got %r"
                % newest["published_display"]
            )
    # A trailing Z is valid RFC 3339 and must not be treated as unparseable.
    if "EEEEEEEEEEE" not in by_id:
        failures.append("a Z-suffixed timestamp was wrongly dropped")
    elif by_id["EEEEEEEEEEE"]["published_display"] != "September 20, 2026":
        failures.append(
            "Z-suffixed timestamp formatted wrongly, got %r"
            % by_id["EEEEEEEEEEE"]["published_display"]
        )

    # No entry may carry the internal sort key into the data file.
    if any("_sort_key" in v for v in videos):
        failures.append("internal sort key leaked into the output")

    # An empty or video-less feed must produce nothing, so that main() refuses
    # to overwrite a good snapshot with it.
    empty = parse_feed(b'<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom"/>')
    if empty:
        failures.append("an entry-less feed should parse to no videos, got %r" % empty)

    # A truncated response must raise rather than parse to an empty list,
    # which would otherwise look identical to a channel with no videos.
    try:
        parse_feed(b"<feed><entry>")
    except ET.ParseError:
        pass
    else:
        failures.append("malformed XML should raise ParseError")

    # An HTML error page served in place of the feed must not parse as videos.
    try:
        html = parse_feed(b"<html><body><entry>not a feed</entry></body></html>")
    except ET.ParseError:
        pass
    else:
        if html:
            failures.append("an HTML error page should yield no videos, got %r" % html)

    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "youtube_videos.json")
        write_output(path, "UCTEST", videos)
        with open(path, encoding="utf-8") as handle:
            written = json.load(handle)
        if written["channel_id"] != "UCTEST" or len(written["videos"]) != len(videos):
            failures.append("round-trip through write_output lost data: %r" % written)
        if sorted(os.listdir(tmp)) != ["youtube_videos.json"]:
            failures.append("write_output left temporary files behind: %r" % os.listdir(tmp))

    for failure in failures:
        print("self-test FAILED: %s" % failure, file=sys.stderr)
    if failures:
        return 1
    print("self-test passed")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--channel-id", default=DEFAULT_CHANNEL_ID)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    try:
        videos = parse_feed(fetch_feed(args.channel_id))
    except ET.ParseError as error:
        print("feed for %s is not valid XML: %s" % (args.channel_id, error), file=sys.stderr)
        return 1

    if not videos:
        # Every known channel has uploads, so an empty list means the feed
        # changed shape or returned an error page. Keep the snapshot.
        print(
            "feed for %s contained no usable videos; not writing %s"
            % (args.channel_id, args.output),
            file=sys.stderr,
        )
        return 1

    write_output(args.output, args.channel_id, videos)
    print("wrote %d videos to %s" % (len(videos), args.output))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
