"""Pygments lexer for Ratslang.

The lexer does not tokenize on its own. It parses with the tree-sitter-ratslang
grammar and colours the text by the captures of the grammar's highlights.scm,
the same query editors use. The capture names map to Pygments token types, so
any Pygments style applies.
"""
from importlib.resources import files

from pygments.lexer import Lexer
from pygments.token import Comment, Keyword, Name, Number, Operator, Punctuation, String, Text
from tree_sitter import Language, Parser, Query, QueryCursor
import tree_sitter_ratslang

LANGUAGE = Language(tree_sitter_ratslang.language())
HIGHLIGHTS = Query(LANGUAGE, (files("tree_sitter_ratslang.queries") / "highlights.scm").read_text())

# Capture names of highlights.scm. A dotted name falls back to its parent, so
# "constant.builtin.boolean" uses "constant.builtin" when it is not listed.
TOKENS = {
    "comment": Comment,
    "string": String,
    "type": Name.Constant,
    "constant": Name.Constant,
    "constant.builtin": Keyword.Constant,
    "constant.numeric": Number,
    "operator": Operator,
    "punctuation": Punctuation,
    "namespace": Name.Namespace,
    "variable": Name.Variable,
    "variable.builtin": Name.Builtin,
    "keyword": Keyword,
}


def token_for(capture):
    while capture:
        if capture in TOKENS:
            return TOKENS[capture]
        capture = capture.rpartition(".")[0]
    return Text


class RatslangLexer(Lexer):
    name = "Ratslang"
    aliases = ["ratslang", "rl"]
    filenames = ["*.rl"]
    url = "https://ratslang.steado.tech/docs/"

    def get_tokens_unprocessed(self, text):
        data = text.encode()
        tree = Parser(LANGUAGE).parse(data)
        # The first pattern in highlights.scm that captures a node wins, as in
        # Helix. Inner nodes are painted over the nodes that contain them.
        best = {}
        for pattern, captures in QueryCursor(HIGHLIGHTS).matches(tree.root_node):
            for capture, nodes in captures.items():
                for node in nodes:
                    key = (node.start_byte, node.end_byte)
                    if key not in best or pattern < best[key][0]:
                        best[key] = (pattern, token_for(capture))
        painted = [Text] * len(data)
        for (start, end), (_, token) in sorted(best.items(), key=lambda item: item[0][0] - item[0][1]):
            painted[start:end] = [token] * (end - start)
        # Node boundaries fall on character boundaries, so every run decodes.
        position = 0
        start = 0
        for end in range(1, len(data) + 1):
            if end == len(data) or painted[end] is not painted[start]:
                value = data[start:end].decode()
                yield position, painted[start], value
                position += len(value)
                start = end
