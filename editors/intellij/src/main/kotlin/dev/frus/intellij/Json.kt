// JSON for the lines `frus daemon` and `frus run --machine` speak: objects as maps, arrays as
// lists, numbers as doubles. Small and with no dependency, so it does not tie the plugin to a
// library one IDE ships and another does not.

package dev.frus.intellij

object Json {
    /** The value `text` holds, or `null` when it is not JSON (a line that is not a message). */
    fun parseOrNull(text: String): Any? = try {
        Reader(text).document()
    } catch (_: IllegalArgumentException) {
        null
    }

    /** `value` written as one line of JSON: maps, lists, strings, numbers, booleans and null. */
    fun write(value: Any?): String = StringBuilder().also { write(value, it) }.toString()

    private fun write(value: Any?, out: StringBuilder) {
        when (value) {
            null -> out.append("null")
            is String -> quote(value, out)
            is Boolean -> out.append(value)
            is Int, is Long -> out.append(value)
            is Number -> {
                val d = value.toDouble()
                if (d == Math.floor(d) && !d.isInfinite() && Math.abs(d) < 1e15) out.append(d.toLong()) else out.append(d)
            }
            is Map<*, *> -> {
                out.append('{')
                var first = true
                for ((k, v) in value) {
                    if (!first) out.append(',')
                    first = false
                    quote(k.toString(), out)
                    out.append(':')
                    write(v, out)
                }
                out.append('}')
            }
            is Iterable<*> -> {
                out.append('[')
                var first = true
                for (v in value) {
                    if (!first) out.append(',')
                    first = false
                    write(v, out)
                }
                out.append(']')
            }
            else -> quote(value.toString(), out)
        }
    }

    private fun quote(s: String, out: StringBuilder) {
        out.append('"')
        for (c in s) {
            when (c) {
                '"' -> out.append("\\\"")
                '\\' -> out.append("\\\\")
                '\n' -> out.append("\\n")
                '\r' -> out.append("\\r")
                '\t' -> out.append("\\t")
                else -> if (c < ' ') out.append(String.format("\\u%04x", c.code)) else out.append(c)
            }
        }
        out.append('"')
    }

    private class Reader(private val s: String) {
        private var at = 0

        fun document(): Any? {
            val value = value()
            space()
            require(at == s.length) { "trailing characters" }
            return value
        }

        private fun value(): Any? {
            space()
            require(at < s.length) { "end of input" }
            return when (val c = s[at]) {
                '{' -> obj()
                '[' -> array()
                '"' -> string()
                't' -> word("true", true)
                'f' -> word("false", false)
                'n' -> word("null", null)
                else -> if (c == '-' || c.isDigit()) number() else throw IllegalArgumentException("unexpected $c")
            }
        }

        private fun obj(): Map<String, Any?> {
            at++
            val map = LinkedHashMap<String, Any?>()
            space()
            if (peek() == '}') {
                at++
                return map
            }
            while (true) {
                space()
                require(peek() == '"') { "a key" }
                val key = string()
                space()
                require(peek() == ':') { "a colon" }
                at++
                map[key] = value()
                space()
                when (peek()) {
                    ',' -> at++
                    '}' -> {
                        at++
                        return map
                    }
                    else -> throw IllegalArgumentException("a comma or }")
                }
            }
        }

        private fun array(): List<Any?> {
            at++
            val list = ArrayList<Any?>()
            space()
            if (peek() == ']') {
                at++
                return list
            }
            while (true) {
                list.add(value())
                space()
                when (peek()) {
                    ',' -> at++
                    ']' -> {
                        at++
                        return list
                    }
                    else -> throw IllegalArgumentException("a comma or ]")
                }
            }
        }

        private fun string(): String {
            at++
            val out = StringBuilder()
            while (true) {
                require(at < s.length) { "an unterminated string" }
                val c = s[at++]
                when (c) {
                    '"' -> return out.toString()
                    '\\' -> {
                        require(at < s.length) { "an unterminated escape" }
                        when (val e = s[at++]) {
                            '"', '\\', '/' -> out.append(e)
                            'b' -> out.append('\b')
                            'f' -> out.append('\u000c')
                            'n' -> out.append('\n')
                            'r' -> out.append('\r')
                            't' -> out.append('\t')
                            'u' -> {
                                require(at + 4 <= s.length) { "a short \\u escape" }
                                out.append(s.substring(at, at + 4).toInt(16).toChar())
                                at += 4
                            }
                            else -> throw IllegalArgumentException("escape \\$e")
                        }
                    }
                    else -> out.append(c)
                }
            }
        }

        private fun number(): Double {
            val start = at
            while (at < s.length && (s[at].isDigit() || s[at] in "+-.eE")) at++
            return s.substring(start, at).toDoubleOrNull() ?: throw IllegalArgumentException("a number")
        }

        private fun word(word: String, value: Any?): Any? {
            require(s.startsWith(word, at)) { "expected $word" }
            at += word.length
            return value
        }

        private fun peek(): Char? = if (at < s.length) s[at] else null

        private fun space() {
            while (at < s.length && s[at].isWhitespace()) at++
        }
    }
}

/** A field of a JSON object, as a string; `null` when it is missing or not one. */
internal fun Map<*, *>.str(key: String): String? = this[key] as? String

/** A field of a JSON object, as a boolean; `false` when missing. */
internal fun Map<*, *>.bool(key: String): Boolean = this[key] as? Boolean ?: false

/** A field of a JSON object, as an object. */
internal fun Map<*, *>.obj(key: String): Map<*, *>? = this[key] as? Map<*, *>
