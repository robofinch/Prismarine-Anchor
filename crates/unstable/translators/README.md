### Notes on conformance to copyright

This project has been cautious about copyright violations, and is well within the lines
as far as I am aware. (I'm not sure how many people bother to add the license or notice files
required by dependencies.)
Note that this project is allowed to rely on copyrighted information at runtime,
such as Mojang's publicly (and officially) available vanilla resource packs, and Amulet's
PyMCTranslate data in the Universal format. The parsers for Universal data of course have similar
functionality to PyMCTranslate out of necessity, but copyright protects expression, not any ideas;
and the Universal format isn't patented. Redistributing copyrighted data is still prohibited,
so the editor can download the data from its official distributor at compile time, but cannot
have that data compiled into it. (Well, the editor could be modified for personal use and have the
data compiled into it, but under Amulet's license, the editor then likely could't be shared with
others.)

Of course, avoiding copyright violations does not preclude unethical behavior;
I believe I've done my work ethically. I'm not sure what Amulet's reaction will be to projects like
this, so that's the one thing I'm worried about; however, with Amulet currently being free,
and with the utility and importance of open-source projects for the community, I'm comfortable with
my decision. I also put quite a lot of care into (to the greatest extent possible)
not looking at their source code, and reimplementing their functionality myself instead
of relying on their work. (With PyMCTranslate, some of their code had to be read to determine what
the Universal format even is, what with the years-out-of-date documentation.)
