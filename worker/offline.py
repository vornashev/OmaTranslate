#!/usr/bin/env python3
"""Persistent, CPU-only CTranslate2 worker. No PyTorch or network access."""
import hashlib
import json
import pathlib
import re
import select
import sys
import time


LANGUAGE = re.compile(r"[a-z]{2,3}(-[A-Za-z]{2,4})?")


class OfflineError(Exception):
    pass


def verify(path):
    try:
        metadata = json.loads((path / "integrity.json").read_text())
        if not metadata["files"] or "model/model.bin" not in metadata["files"]:
            raise ValueError("Missing model digest")
        for name, expected in metadata["files"].items():
            relative = pathlib.PurePosixPath(name)
            if relative.is_absolute() or ".." in relative.parts:
                raise ValueError("Unsafe member")
            with (path / name).open("rb") as f:
                actual = hashlib.file_digest(f, "sha256").hexdigest()
            if actual != expected:
                raise ValueError("Digest mismatch")
    except (OSError, ValueError, KeyError, TypeError):
        raise OfflineError("invalidModel") from None


class Engine:
    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.models = {}

    def load(self, source, target):
        if not all(LANGUAGE.fullmatch(code) for code in (source, target)):
            raise OfflineError("unsupportedLanguage")
        pair = source + "_" + target
        if pair in self.models:
            return self.models[pair]
        path = self.root / pair
        if not (path / "integrity.json").is_file():
            raise OfflineError("modelMissing")
        try:
            import ctranslate2
            import sentencepiece
        except ImportError:
            raise OfflineError("offlineUnavailable") from None
        verify(path)
        try:
            translator = ctranslate2.Translator(str(path / "model"), device="cpu", compute_type="int8", inter_threads=1, intra_threads=4)
            tokenizer = sentencepiece.SentencePieceProcessor(model_file=str(path / "sentencepiece.model"))
        except Exception:
            raise OfflineError("invalidModel") from None
        self.models[pair] = translator, tokenizer
        return self.models[pair]

    def translate(self, request):
        source, target = request.get("source", "en"), request.get("target", "ru")
        if source == target:
            return request["segments"]
        translator, tokenizer = self.load(source, target)
        if request.get("keepModelsWarm"):
            if (self.root / (target + "_" + source) / "integrity.json").exists():
                self.load(target, source)
        output = []
        for text in request["segments"]:
            # Keep paragraph boundaries and split overlong sentences at token boundaries.
            paragraphs = []
            for paragraph in text.split("\n"):
                if not paragraph.strip():
                    paragraphs.append(paragraph)
                    continue
                sentences = re.split(r"(?<=[.!?。！？])\s*(?=[^\W\d_])", paragraph) if source.startswith(("ja", "zh")) else re.split(r"(?<=[.!?])\s+(?=[^\W\d_])", paragraph)
                tokens = [tokenizer.encode(s, out_type=str) for s in sentences]
                batches = [t[i:i + 256] for t in tokens for i in range(0, len(t), 256)]
                results = translator.translate_batch(batches, beam_size=2, max_batch_size=512,
                                                     batch_type="tokens", replace_unknowns=True,
                                                     max_decoding_length=512, length_penalty=0.2)
                paragraphs.append(" ".join(tokenizer.decode(r.hypotheses[0]).strip() for r in results))
            output.append("\n".join(paragraphs))
        return output


def main():
    engine = Engine(sys.argv[1])
    print(json.dumps({"event": "ready"}), flush=True)
    warm = False
    last = time.monotonic()
    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 30)
        if not readable:
            if not warm and time.monotonic() - last > 600:
                engine.models.clear()
            continue
        line = sys.stdin.buffer.readline(1048577)
        if not line:
            return
        if len(line) > 1048576:
            return
        try:
            request = json.loads(line)
            warm = bool(request.get("keepModelsWarm"))
            result = {"translations": engine.translate(request)}
        except OfflineError as error:
            result = {"error": str(error)}
        except Exception:
            result = {"error": "offlineFailed"}
        last = time.monotonic()
        print(json.dumps(result, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
