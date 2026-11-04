#!/usr/bin/env python3
"""Read-only semantic retrieval and grounded Qwen answering on an isolated demo .tme state.
Tested concepts: paraphrase retrieval, grounded answer, irrelevant-query handling.
This is an experimental quality probe, not proof of reliable long-term memory.
"""
import json
import math
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
QUESTIONS = [
    ('Wer hat den kaputten Zeitmesser wieder funktionsfähig gemacht?', 'Uhrmacher'),
    ('An welchem Ort entdeckte Mira die Holzkiste?', 'Dachboden'),
    ('Was geschah, als der Zeitmesser wieder funktionierte?', 'Fenster'),
]

def post(endpoint, payload):
    req = urllib.request.Request(BASE + endpoint,
        data=json.dumps(payload, ensure_ascii=False).encode('utf-8'),
        headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=240) as reply:
        return json.load(reply)

def embed(text):
    vec = post('/api/embed', {'model':'mxbai-embed-large:latest', 'input':text})['embeddings'][0]
    if not vec or not all(isinstance(x,(int,float)) and math.isfinite(x) for x in vec):
        raise ValueError('Invalid embedding')
    return vec

def chat(system, user):
    raw = post('/api/chat', {
        'model':'qwen3:8b','stream':False,'format':'json',
        'options':{'temperature':0},
        'messages':[{'role':'system','content':system},{'role':'user','content':user}],
    })
    return json.loads(raw['message']['content'])

def run_bridge(*args):
    cmd = ['cargo','run','--quiet','--bin','text-ingestion-bridge','--', *map(str,args)]
    result = subprocess.run(cmd,cwd=ROOT,check=True,capture_output=True,text=True)
    return result.stdout.strip()

print('=== SEMANTIC RETRIEVAL / QWEN → TME ===',flush=True)
segmentation = chat(
    'Teile den folgenden Text in logische, chronologisch geordnete Sinnabschnitte. '
    'Gib ausschließlich JSON mit title:string und segments:[{text:string}]. '
    'Bewahre alle Fakten, erfinde nichts.', STORY)
segments = segmentation['segments']
title = segmentation['title']
if not isinstance(title,str) or not title.strip() or not isinstance(segments,list) or not (1 <= len(segments) <= 128):
    raise ValueError('Invalid segmentation')
anchor_vec = embed(title)
prepared=[]
for i, entry in enumerate(segments, 1):
    text = entry['text']
    if not isinstance(text,str) or not text.strip():
        raise ValueError('Invalid segment text')
    vector = embed(text)
    if len(vector) != len(anchor_vec):
        raise ValueError('Vector dimension mismatch')
    prepared.append({'id':f'mira-{i:02d}', 'text':text,'vector':vector})

with tempfile.TemporaryDirectory(prefix='tme-search-demo-') as folder:
    root = Path(folder)
    state_id = str(uuid.uuid4())
    serial = 'TEXTDEMO'
    doc = {'anchor_id':'mira-story','title':title,'anchor_vector':anchor_vec,
           'source':'demo/mira-story-v1','segments':prepared}
    payload = root/'document.json'
    payload.write_text(json.dumps(doc,ensure_ascii=False),encoding='utf-8')
    print('CREATE:', run_bridge('create',root,state_id,serial,payload), flush=True)
    matched = 0
    for question, expected in QUESTIONS:
        query_file = root/'search.json'
        query_file.write_text(json.dumps({
            'anchor_id':'mira-story','query_vector':embed(question),
            'max_depth':3,'limit':20,
        }),encoding='utf-8')
        search = json.loads(run_bridge('search',root,state_id,serial,query_file))
        results = search['results']
        if not results:
            raise RuntimeError('No relevant retrieval candidates returned')
        evidence = '\n'.join(f"[Token {r['token_id']}; source={r['source']}] {r['text']}"
                             for r in results[:2])
        answer = chat(
            'Beantworte ausschließlich anhand der gegebenen Erinnerungen. '
            'Wenn die Antwort daraus nicht hervorgeht, antworte UNKNOWN. '
            'Antworte ausschließlich als JSON mit answer:string.',
            f'ERINNERUNGEN:\n{evidence}\n\nFRAGE: {question}')['answer']
        source_text = ' '.join(r['text'] for r in results)
        retrieved = expected.casefold() in source_text.casefold()
        matched += int(retrieved)
        print('\nQUESTION:', question)
        print('FULL RANKING:')
        for rank, r in enumerate(results, 1):
            print(
                rank,
                r['token_id'],
                'position=', r['position'],
                'similarity=', round(r['similarity'], 4),
                'path=', round(r['path_weight'], 4),
                'score=', round(r['combined_score'], 4),
                'contains_expected=', expected.casefold() in r['text'].casefold(),
                'text=', r['text'][:150]
            )
        print('TOP RESULT:', results[0]['token_id'],
              'similarity=', round(results[0]['similarity'], 3))
        print('EXPECTED KEYWORD IN TOP 3:',retrieved)
        print('ANSWER:',answer)
    print(f'RETRIEVAL_KEYWORD_COVERAGE {matched}/{len(QUESTIONS)}',flush=True)
    print('SEMANTIC_RETRIEVAL_PIPELINE_OK (temporary state cleaned up)',flush=True)
