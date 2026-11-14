$ cooee theme check packs/themes/farm
```
ok   packs/themes/farm
exit=0
```
$ cooee theme check packs/themes/kitchen
```
ok   packs/themes/kitchen
exit=0
```
$ cooee theme check packs/themes/garden
```
ok   packs/themes/garden
exit=0
```
$ cooee theme check packs/starter
```
ok   packs/starter
exit=0
```
case1: sprites.svg += <script>alert(1)</script>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case1
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case1
     sprites.svg: <script> (active content)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case2: scene.svg += <g onload="alert(1)"/>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case2
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case2
     scene.svg: an on* attribute (onload)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case3: sprites.svg += <foreignObject><div/></foreignObject>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case3
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case3
     sprites.svg: <foreignObject> (active content)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case4: scene.svg += <use xlink:href="https://evil.example/x.svg#a"/>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case4
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case4
     scene.svg: a non-local href "https://evil.example/x.svg#a" (only #id)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case5: sprites.svg += <a href="javascript:alert(1)"><rect/></a>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case5
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case5
     sprites.svg: a non-local href "javascript:alert(1)" (only #id)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case6: sprites.svg += <image href="data:image/svg+xml;base64,AAAA"/>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case6
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case6
     sprites.svg: a non-local href "data:image/svg+xml;base64,AAAA" (only #id)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case7: sprites.svg += <!-- <script>x</script> -->
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case7
```
ok   /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case7
exit=0
```
case8: sprites.svg += <![CDATA[ <!-- ]]><script>alert(1)</script><![CDATA[ --> ]]>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case8
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case8
     sprites.svg: <script> (active content)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case9: scene.svg += <?x <!-- ?><script>alert(1)</script><?x --> ?>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case9
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case9
     scene.svg: <script> (active content)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case10: sprites.svg += <use href="#s-edit"/>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case10
```
ok   /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case10
exit=0
```
case11: sprites.svg += <SCRIPT>alert(1)</SCRIPT>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case11
```
ok   /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case11
exit=0
```
case12: sprites.svg += <svg:script>alert(1)</svg:script>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case12
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case12
     sprites.svg: <script> (active content)
cooee: 1 theme(s) fail the theme gate
exit=1
```
case13: scene.svg += <g ONCLICK="x"/>
$ cooee theme check /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case13
```
FAIL /Users/jkmacmini/.local/share/drover/work/cooee/verify-319-r5/shots/packs/case13
     scene.svg: an on* attribute (onclick)
cooee: 1 theme(s) fail the theme gate
exit=1
```
