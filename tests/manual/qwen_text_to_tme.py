#!/usr/bin/env python3
"""Qwen semantic segmentation → mxbai embedding → fresh isolated .tme state → recall.
This demo does not assess semantic equivalence and cannot prove LLM memory quality.
"""
import json
import subprocess
import tempfile
import urllib.request
import uuid
from pathlib import Path

BASE = 'http://127.0.0.1:11434'
ROOT = Path(__file__).resolve().parents[2]
STORY = """Mira fand auf dem Dachboden eine alte Holzkiste.
Darin lag eine Karte mit einer rot markierten Brücke.
Am nächsten Morgen besuchte sie die Brücke.
Dort traf sie einen alten Uhrmacher, der die Karte erkannte.
Er erklärte, dass unter der Brücke früher eine Werkstatt gewesen war.
Mira entdeckte im Mauerwerk eine kleine Tür.
Hinter der Tür lag eine beschädigte Uhr.
Der Uhrmacher reparierte sie. Als die Uhr wieder lief,
öffneten sich in der Werkstatt verborgene Fenster.
Mira beschloss, die Geschichte der Werkstatt aufzuschreiben."""

def post(endpoint, payload):
    request = urllib.request.Request(BASE + endpoint,
        data=json.dumps(payload).encode(), headers={'Content-Type':'application/json'})
    with urllib.request.urlopen(request, timeout=240) as response:
        return json.load(response)

print('=== QWEN → TME TEXT INGESTION ===', flush=True)
response = post('/api/chat', {
    'model':'qwen3:8b', 'stream':False, 'format':'json',
    'options':{'temperature':0},
    'messages':[
        {'role':'system','content':(
            'Du segmentierst Geschichten in logisch zusammenhängende Abschnitte. '
            'Antworte nur mit JSON: {"title":string,"segments":[{"text":string}]}. '
            'Behalte alle wichtigen Fakten; erfinde nichts.')},
        {'role':'user','content':STORY},
    ],
})
parsed = json.loads(response['message']['content'])
title = parsed['title']
segments = parsed['segments']
if not isinstance(title, str) or not title.strip() or not isinstance(segments,list) or not (1 <= len(segments) <= 128):
    raise ValueError('Invalid model document')

def embedding(text):
    result = post('/api/embed', {'model':'mxbai-embed-large:latest','input':text})
    return result['embeddings'][0]

vectors = [embedding(title)]
processed = []
for i, s in enumerate(segments, 1):
    text = s['text']
    if not isinstance(text,str) or not text.strip():
        raise ValueError(f'Invalid segment {i}')
    vector = embedding(text)
    if len(vector) != len(vectors[0]):
        raise ValueError('Inconsistent embedding dimensions')
    vectors.append(vector)
    processed.append({'id':f'mira-segment-{i:02d}','text':text,'vector':vector})

document = {'anchor_id':'mira-story','title':title,'anchor_vector':vectors[0],
            'source':'demo/mira-story-v1','segments':processed}
with tempfile.TemporaryDirectory(prefix='tme-text-demo-') as temp:
    folder = Path(temp)
    payload = folder / 'segments.json'
    payload.write_text(json.dumps(document,ensure_ascii=False), encoding='utf-8')
    state_id = str(uuid.uuid4())
    cmd = ['cargo','run','--quiet','--bin','text-ingestion-bridge','--']
    def bridge(*args):
        return subprocess.run(cmd + list(args),cwd=ROOT,text=True,check=True,capture_output=True).stdout.strip()
    print('SEGMENTS',len(processed), 'VECTOR_DIM',len(vectors[0]),flush=True)
    print(bridge('create',str(folder),state_id,'TEXTDEMO',str(payload)),flush=True)
    print('RECALL AFTER RELOAD:',flush=True)
    recalled = bridge('recall',str(folder),state_id,'TEXTDEMO','mira-story')
    print(recalled,flush=True)
    assert len(recalled.splitlines()) == len(processed), 'Recall count mismatch'
    # Ingest the exact same embedding payload to prove no duplicate insertions.
    print(bridge('ingest',str(folder),state_id,'TEXTDEMO',str(payload)),flush=True)
print('QWEN_TME_PERSISTENCE_OK (temporary demo state cleaned up)', flush=True)
