import json
import urllib.request

BASE = "http://127.0.0.1:11434"

story = """
Mira fand auf dem Dachboden eine alte Holzkiste.
Darin lag eine Karte mit einer rot markierten Brücke.
Am nächsten Morgen besuchte sie die Brücke.
Dort traf sie einen alten Uhrmacher, der die Karte erkannte.
Er erklärte, dass unter der Brücke früher eine Werkstatt
gewesen war. Mira entdeckte im Mauerwerk eine kleine Tür.
Hinter der Tür lag eine beschädigte Uhr.
Der Uhrmacher reparierte sie. Als die Uhr wieder lief,
öffneten sich in der Werkstatt verborgene Fenster.
Mira beschloss, die Geschichte der Werkstatt aufzuschreiben.
"""

def post(path, payload):
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        BASE + path,
        data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=240) as response:
        return json.load(response)

print("=== QWEN SEMANTIC SEGMENTATION ===", flush=True)

result = post("/api/chat", {
    "model": "qwen3:8b",
    "stream": False,
    "format": "json",
    "options": {"temperature": 0},
    "messages": [
        {
            "role": "system",
            "content": (
                "Segmentiere den Text in sinnvolle "
                "chronologisch geordnete Abschnitte. "
                "Antworte ausschließlich als JSON-Objekt "
                "mit 'title' und 'segments'. "
                "Jedes Segment enthält 'text'. "
                "Erfinde keine Informationen."
            ),
        },
        {"role": "user", "content": story},
    ],
})

content = result["message"]["content"]
parsed = json.loads(content)

segments = parsed["segments"]
if not isinstance(segments, list) or not segments:
    raise ValueError("Keine Segmente erzeugt")

print("TITEL:", parsed.get("title", ""))
print("SEGMENTE:", len(segments))

for index, segment in enumerate(segments, 1):
    text = segment["text"]
    if not isinstance(text, str) or not text.strip():
        raise ValueError("Ungueltiger Segmenttext")

    embedding = post("/api/embed", {
        "model": "mxbai-embed-large:latest",
        "input": text,
    })["embeddings"][0]

    if not embedding:
        raise ValueError("Leerer Embedding-Vektor")

    print(f"\nTOKEN {index}")
    print("TEXT:", text)
    print("VECTOR DIM:", len(embedding))

print("\nQWEN_TEXT_PROBE_OK")
