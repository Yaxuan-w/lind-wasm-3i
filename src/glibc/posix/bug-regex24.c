#include <regex.h>
#include <stdio.h>
#include <string.h>
#include <libc-diag.h>

#define str "civic"

#define N 10
static const char *expected[N] =
  {
    str, "c", "i", "", "", "", "", "", "", ""
  };

static int
do_test (void)
{
  regex_t rbuf;
  static const char pat[] = "\
^(.?)(.?)(.?)(.?)(.?)(.?)(.?)(.?)(.?).?\\9\\8\\7\\6\\5\\4\\3\\2\\1$";

  int err = regcomp (&rbuf, pat, REG_EXTENDED);
  if (err != 0)
    {
      char errstr[300];
      regerror (err, &rbuf, errstr, sizeof (errstr));
      puts (errstr);
      return err;
    }

  regmatch_t m[N];
  err = regexec (&rbuf, str, N, m, 0);
  if (err != 0)
    {
      puts ("regexec failed");
      return 1;
    }

  int result = 0;
  for (int i = 0; i < N; ++i)
    if (m[i].rm_so == -1)
      {
	printf ("m[%d] unused\n", i);
	result = 1;
      }
    else
      {
	int len = m[i].rm_eo - m[i].rm_so;

	/* clang complains that adding a 'regoff_t' to a string does not
	   append to it, and the printf idea below is to make rm_so as
	   an offset to str.  */
	DIAG_PUSH_NEEDS_COMMENT_CLANG;
	DIAG_IGNORE_NEEDS_COMMENT_CLANG (13, "-Wstring-plus-int");
	printf ("m[%d] = \"%.*s\"\n", i, len, str + m[i].rm_so);

	if (strlen (expected[i]) != len
	    || memcmp (expected[i], str + m[i].rm_so, len) != 0)
	  result = 1;
	DIAG_POP_NEEDS_COMMENT_CLANG;
      }

  return result;
}

#define TIMEOUT 30
#define TEST_FUNCTION do_test ()
#include "../test-skeleton.c"
