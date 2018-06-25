// Copyright 2016 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/* global mat4, WGLUProgram */

window.VRCubeSea = (function () {
  "use strict";

  var cubeSeaVS = [
    "uniform mat4 projectionMat;",
    "uniform mat4 modelViewMat;",
    "uniform mat3 normalMat;",
    "attribute vec3 position;",
    "attribute vec2 texCoord;",
    "attribute vec4 color;",
    "attribute vec3 normal;",
    "varying vec2 vTexCoord;",
    "varying vec4 vColor;",
    "varying vec3 vLight;",

    "const vec3 lightDir = vec3(0.75, 0.5, 1.0);",
    "const vec3 ambientColor = vec3(0.5, 0.5, 0.5);",
    "const vec3 lightColor = vec3(0.75, 0.75, 0.75);",

    "void main() {",
    "  vec3 normalRotated = normalMat * normal;",
    //"  float lightFactor = max(dot(normalize(lightDir), normalRotated), 0.0);",
    //"  vLight = ambientColor + (lightColor * lightFactor);",
    "  vTexCoord = texCoord;",
    "  vColor = color;",
    "  gl_Position = modelViewMat * vec4( position, 1.0 );",
    "}",
  ].join("\n");

  var cubeSeaFS = [
    "precision mediump float;",
    "uniform sampler2D diffuse;",
    "varying vec2 vTexCoord;",
    "varying vec4 vColor;",
    "varying vec3 vLight;",

    "void main() {",
    //"  gl_FragColor = vec4(vLight, 1.0) * texture2D(diffuse, vTexCoord);",
    "  gl_FragColor = texture2D(diffuse, vTexCoord);",
    //"  gl_FragColor = vec4(0.5, 0.9, 0.5, 1.0);",
    //"  gl_FragColor = vColor;",
    "}",
  ].join("\n");

  // Used when we want to stress the GPU a bit more.
  // Stolen with love from https://www.clicktorelease.com/code/codevember-2016/4/
  var heavyCubeSeaFS = [
    "precision mediump float;",

    "uniform sampler2D diffuse;",
    "varying vec2 vTexCoord;",
    "varying vec3 vLight;",

    "vec2 dimensions = vec2(64, 64);",
    "float seed = 0.42;",

    "vec2 hash( vec2 p ) {",
    "  p=vec2(dot(p,vec2(127.1,311.7)),dot(p,vec2(269.5,183.3)));",
    "  return fract(sin(p)*18.5453);",
    "}",

    "vec3 hash3( vec2 p ) {",
    "    vec3 q = vec3( dot(p,vec2(127.1,311.7)),",
    "           dot(p,vec2(269.5,183.3)),",
    "           dot(p,vec2(419.2,371.9)) );",
    "  return fract(sin(q)*43758.5453);",
    "}",

    "float iqnoise( in vec2 x, float u, float v ) {",
    "  vec2 p = floor(x);",
    "  vec2 f = fract(x);",
    "  float k = 1.0+63.0*pow(1.0-v,4.0);",
    "  float va = 0.0;",
    "  float wt = 0.0;",
    "  for( int j=-2; j<=2; j++ )",
    "    for( int i=-2; i<=2; i++ ) {",
    "      vec2 g = vec2( float(i),float(j) );",
    "      vec3 o = hash3( p + g )*vec3(u,u,1.0);",
    "      vec2 r = g - f + o.xy;",
    "      float d = dot(r,r);",
    "      float ww = pow( 1.0-smoothstep(0.0,1.414,sqrt(d)), k );",
    "      va += o.z*ww;",
    "      wt += ww;",
    "    }",
    "  return va/wt;",
    "}",

    "// return distance, and cell id",
    "vec2 voronoi( in vec2 x ) {",
    "  vec2 n = floor( x );",
    "  vec2 f = fract( x );",
    "  vec3 m = vec3( 8.0 );",
    "  for( int j=-1; j<=1; j++ )",
    "    for( int i=-1; i<=1; i++ ) {",
    "      vec2  g = vec2( float(i), float(j) );",
    "      vec2  o = hash( n + g );",
    "      vec2  r = g - f + (0.5+0.5*sin(seed+6.2831*o));",
    "      float d = dot( r, r );",
    "      if( d<m.x )",
    "        m = vec3( d, o );",
    "    }",
    "  return vec2( sqrt(m.x), m.y+m.z );",
    "}",

    "void main() {",
    "  vec2 uv = ( vTexCoord );",
    "  uv *= vec2( 10., 10. );",
    "  uv += seed;",
    "  vec2 p = 0.5 - 0.5*sin( 0.*vec2(1.01,1.71) );",

    "  vec2 c = voronoi( uv );",
    "  vec3 col = vec3( c.y / 2. );",

    "  float f = iqnoise( 1. * uv + c.y, p.x, p.y );",
    "  col *= 1.0 + .25 * vec3( f );",

    "  gl_FragColor = vec4(vLight, 1.0) * texture2D(diffuse, vTexCoord) * vec4( col, 1. );",
    "}"
  ].join("\n");

  var CubeSea = function (gl, texture, gridSize, cubeScale, heavy, halfOnly, autorotate) {
    this.gl = gl;

    if (!gridSize) {
      gridSize = 10;
    }

    this.statsMat = mat4.create();
    this.normalMat = mat3.create();
    this.heroRotationMat = mat4.create();
    this.heroModelViewMat = mat4.create();
    this.autoRotationMat = mat4.create();
    this.cubesModelViewMat = mat4.create();

    this.texture = texture;

    this.program = new WGLUProgram(gl);
    this.program.attachShaderSource(cubeSeaVS, gl.VERTEX_SHADER);
    this.program.attachShaderSource(heavy ? heavyCubeSeaFS :cubeSeaFS, gl.FRAGMENT_SHADER);
    this.program.bindAttribLocation({
      position: 0,
      texCoord: 1,
      color: 2,
      normal: 3
    });
    this.program.link();

    this.autorotate = autorotate;

    var cubeVerts = [];
    var cubeIndices = [];

    // Build a single cube.
    function appendCube (x, y, z, size) {
      if (!size) size = 0.2;
      if (cubeScale) size *= cubeScale;
      // Bottom
      var idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 1, idx + 2);
      cubeIndices.push(idx, idx + 2, idx + 3);

      //             X         Y         Z         U    V    R    G    B    A    NX    NY   NZ
      cubeVerts.push(x - size, y - size, z - size, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
      cubeVerts.push(x + size, y - size, z - size, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0);
      cubeVerts.push(x + size, y - size, z + size, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
      cubeVerts.push(x - size, y - size, z + size, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0);

      // Top
      idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 2, idx + 1);
      cubeIndices.push(idx, idx + 3, idx + 2);

      cubeVerts.push(x - size, y + size, z - size, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
      cubeVerts.push(x + size, y + size, z - size, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0);
      cubeVerts.push(x + size, y + size, z + size, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
      cubeVerts.push(x - size, y + size, z + size, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0);

      // Left
      idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 2, idx + 1);
      cubeIndices.push(idx, idx + 3, idx + 2);

      cubeVerts.push(x - size, y - size, z - size, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, -1.0, 0.0, 0.0);
      cubeVerts.push(x - size, y + size, z - size, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, -1.0, 0.0, 0.0);
      cubeVerts.push(x - size, y + size, z + size, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0);
      cubeVerts.push(x - size, y - size, z + size, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0);

      // Right
      idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 1, idx + 2);
      cubeIndices.push(idx, idx + 2, idx + 3);

      cubeVerts.push(x + size, y - size, z - size, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0);
      cubeVerts.push(x + size, y + size, z - size, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0);
      cubeVerts.push(x + size, y + size, z + size, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0);
      cubeVerts.push(x + size, y - size, z + size, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0);

      // Back
      idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 2, idx + 1);
      cubeIndices.push(idx, idx + 3, idx + 2);

      cubeVerts.push(x - size, y - size, z - size, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0);
      cubeVerts.push(x + size, y - size, z - size, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, -1.0);
      cubeVerts.push(x + size, y + size, z - size, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, -1.0);
      cubeVerts.push(x - size, y + size, z - size, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0);

      // Front
      idx = cubeVerts.length / 12.0;
      cubeIndices.push(idx, idx + 1, idx + 2);
      cubeIndices.push(idx, idx + 2, idx + 3);

      cubeVerts.push(x - size, y - size, z + size, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
      cubeVerts.push(x + size, y - size, z + size, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0);
      cubeVerts.push(x + size, y + size, z + size, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0);
      cubeVerts.push(x - size, y + size, z + size, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
    }

    // Build the cube sea
    var N = 10;
    for(var i = 0; i < N; i++){
        var theta = 2 * Math.PI * i / N;
      //appendCube(5*Math.cos(theta), 0, 5*Math.sin(theta), 1.0);

    }
      appendCube(0, 0, 0, 0.5);
    for (var x = 0; x < gridSize; ++x) {
      for (var y = 0; y < gridSize; ++y) {
        for (var z = 0; z < gridSize; ++z) {
          //appendCube(x - (gridSize / 2), y - (gridSize / 2), z - (gridSize / 2));
        }
      }
    }

    this.indexCount = cubeIndices.length;

    // Add some "hero cubes" for separate animation.
    this.heroOffset = cubeIndices.length;
    appendCube(0, 0.25, -0.8, 0.05);
    appendCube(0.8, 0.25, 0, 0.05);
    appendCube(0, 0.25, 0.8, 0.05);
    appendCube(-0.8, 0.25, 0, 0.05);
    this.heroCount = cubeIndices.length - this.heroOffset;

    this.vertBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(cubeVerts), gl.STATIC_DRAW);

    this.indexBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.indexBuffer);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array(cubeIndices), gl.STATIC_DRAW);

    this.boardVert = [];
    var boardSize = 20.0;
    // left top
    //                  X    Y     Z           U    V    R    G    B    A    NX    NY   NZ
    this.boardVert.push(0.0, 10.0, -boardSize, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
    // left bottom
    this.boardVert.push(0.0, 0.0, -boardSize, 0.0, 0.5, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
    // right bottom
    this.boardVert.push(0.0, 0.0, boardSize, 1.0, 0.5, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);

    // left top
    //                  X    Y     Z           U    V    R    G    B    A    NX    NY   NZ
    this.boardVert.push(0.0, 10.0, -boardSize, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
    // right bottom
    this.boardVert.push(0.0, 0.0, boardSize, 1.0, 0.5, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);
    // right top
    this.boardVert.push(0.0, 10.0, boardSize, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0);

    this.boardVertBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.boardVertBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(this.boardVert), gl.STATIC_DRAW);

    this.textureCanvas = document.createElement("canvas");
    this.textureCanvas.width = 1024 * 2;
    this.textureCanvas.height = 1024 * 2;

    this.canvasTexture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.canvasTexture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.textureCanvas);
  };

  function digit(i, n){
    if(n == 3 && i < 10){
      return "00" + i;
    }
    if(n == 3 && i < 100){
      return "0" + i;
    }
    if(n == 2 && i < 10){
      return "0" + i;
    }
    return "" + i
  }

  CubeSea.prototype.render = function (projectionMat, modelViewMat, stats, timestamp, orientation, position) {
    var gl = this.gl;
    var program = this.program;

    program.use();

    //if (this.autorotate && timestamp) {
    //  mat4.fromRotation(this.autoRotationMat, timestamp / 500, [0, -1, 0]);
    //  mat4.multiply(this.cubesModelViewMat, modelViewMat, this.autoRotationMat);
    //  mat3.fromMat4(this.normalMat, this.autoRotationMat);
    //} else {
    this.cubesModelViewMat = modelViewMat;
    mat3.identity(this.normalMat);
    //}

    gl.uniformMatrix4fv(program.uniform.projectionMat, false, projectionMat);
    gl.uniformMatrix3fv(program.uniform.normalMat, false, this.normalMat);

    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertBuffer);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.indexBuffer);

    gl.enableVertexAttribArray(program.attrib.position);
    gl.enableVertexAttribArray(program.attrib.texCoord);
    gl.enableVertexAttribArray(program.attrib.color);
    gl.enableVertexAttribArray(program.attrib.normal);

    gl.vertexAttribPointer(program.attrib.position, 3, gl.FLOAT, false, 48, 0);
    gl.vertexAttribPointer(program.attrib.texCoord, 2, gl.FLOAT, false, 48, 12);
    gl.vertexAttribPointer(program.attrib.color, 4, gl.FLOAT, false, 48, 20);
    gl.vertexAttribPointer(program.attrib.normal, 3, gl.FLOAT, false, 48, 36);

    gl.activeTexture(gl.TEXTURE0);
    gl.uniform1i(this.program.uniform.diffuse, 0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);

    var N = 10;
    for(var i = 0; i < N; i++){
      var theta = 2 * Math.PI * i / N;

      var mm = mat4.create();
      mat4.translate(mm, mm, [5 * Math.cos(theta), 0, 5* Math.sin(theta)]);
      var tran = mat4.clone(mm);
      mat4.mul(mm, modelViewMat, mm);
      mat4.mul(mm, projectionMat, mm);
      gl.uniformMatrix4fv(program.uniform.modelViewMat, false, mm);
      if(i == 0){
        //console.log(theta);
        //console.log("rotate:");
        //console.log(orientation);
        //console.log(modelViewMat);
        ////console.log(projectionMat);
        //console.log(mm);
      }
      gl.drawElements(gl.TRIANGLES, this.indexCount, gl.UNSIGNED_SHORT, 0);
    }

    var mm = mat4.create();
    mat4.translate(mm, mm, [20, 3, 0]);
    var rot = mat4.create();
    mat4.rotate(rot, rot, Math.PI / 2, [0, 1, 0]);
    mat4.mul(mm, rot, mm);
    mat4.mul(mm, modelViewMat, mm);
    mat4.mul(mm, projectionMat, mm);
    gl.uniformMatrix4fv(program.uniform.modelViewMat, false, mm);

    if(position){
        var fontSize = 60;
      var context = this.textureCanvas.getContext("2d");
      context.save();
      context.fillRect(0, 0, this.textureCanvas.width, this.textureCanvas.height);
      context.fillStyle = '#FFFFFF';
      context.font = ""+fontSize+"px 'Arial'";
      context.textAlign = 'left';
      context.textBaseline = 'middle';

      if(orientation){
        var line = 58;
        var x = 200;
        var pos = 20;
        function drawText(text){
          context.fillText(text, x, pos += line);
        }

        var testMat = mat4.create();
        mat4.fromQuat(testMat, orientation);

        drawText(testMat[0].toFixed(3) + " " + testMat[1].toFixed(3) + " " + testMat[2].toFixed(3) + " " + testMat[3].toFixed(3));
        drawText(testMat[4].toFixed(3) + " " + testMat[5].toFixed(3) + " " + testMat[6].toFixed(3) + " " + testMat[7].toFixed(3));
        drawText(testMat[8].toFixed(3) + " " + testMat[9].toFixed(3) + " " + testMat[10].toFixed(3) + " " + testMat[11].toFixed(3));
        drawText(testMat[12].toFixed(3) + " " + testMat[13].toFixed(3) + " " + testMat[13].toFixed(3) + " " + testMat[15].toFixed(3));
        pos += line;
        drawText("Rotation(x,y,z,w):");
        drawText("" + orientation[0].toFixed(6) + " " + orientation[1].toFixed(6) + " " + orientation[2].toFixed(6) + " " + orientation[3].toFixed(6) + " ");
        pos += line;
        drawText("Position(x,y,z):");
        drawText("" + position[0].toFixed(6) + " " + position[1].toFixed(3) + " " + position[2].toFixed(3));

        var button = "";
        for(var i = 0; i < navigator.getGamepads().length; i++){
            var s = navigator.getGamepads()[i];
            button += " "
            for(var j = 0; j < s.buttons.length; j++){
                button += " " + s.buttons[j].pressed;
            }
        }
        drawText("t1:" + window.test  + " t2:" + window.test2 + "" + " button:" + button);

        var date = new Date();
        drawText(digit(date.getHours(), 2) + ":" + digit(date.getMinutes(), 2)
          + ":" + digit(date.getSeconds(), 2) + "." + digit(date.getMilliseconds(), 3) + "");

        x = 600;
        pos = 20;
        drawText(modelViewMat[0].toFixed(3) + " " + modelViewMat[1].toFixed(3) + " " + modelViewMat[2].toFixed(3) + " " + modelViewMat[3].toFixed(3));
        drawText(modelViewMat[4].toFixed(3) + " " + modelViewMat[5].toFixed(3) + " " + modelViewMat[6].toFixed(3) + " " + modelViewMat[7].toFixed(3));
        drawText(modelViewMat[8].toFixed(3) + " " + modelViewMat[9].toFixed(3) + " " + modelViewMat[10].toFixed(3) + " " + modelViewMat[11].toFixed(3));
        drawText(modelViewMat[12].toFixed(3) + " " + modelViewMat[13].toFixed(3) + " " + modelViewMat[13].toFixed(3) + " " + modelViewMat[15].toFixed(3));

      }else{
        context.fillText("Hello world!", 0, 40);
      }
      context.restore();

      gl.activeTexture(gl.TEXTURE0);
      gl.uniform1i(this.program.uniform.diffuse, 0);
      gl.bindTexture(gl.TEXTURE_2D, this.canvasTexture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.textureCanvas);
      gl.generateMipmap(gl.TEXTURE_2D);
    }

    gl.activeTexture(gl.TEXTURE0);
    gl.uniform1i(this.program.uniform.diffuse, 0);
    gl.bindTexture(gl.TEXTURE_2D, this.canvasTexture);

    gl.bindBuffer(gl.ARRAY_BUFFER, this.boardVertBuffer);
    gl.enableVertexAttribArray(program.attrib.position);
    gl.enableVertexAttribArray(program.attrib.texCoord);
    gl.enableVertexAttribArray(program.attrib.color);
    gl.enableVertexAttribArray(program.attrib.normal);

    gl.vertexAttribPointer(program.attrib.position, 3, gl.FLOAT, false, 48, 0);
    gl.vertexAttribPointer(program.attrib.texCoord, 2, gl.FLOAT, false, 48, 12);
    gl.vertexAttribPointer(program.attrib.color, 4, gl.FLOAT, false, 48, 20);
    gl.vertexAttribPointer(program.attrib.normal, 3, gl.FLOAT, false, 48, 36);

    gl.drawArrays(gl.TRIANGLES, 0, this.boardVert.length / 12);

    if(!window.kei){
      window.kei = 1.0;
    }
    mm = mat4.create();
    mat4.translate(mm, mm, [20, 3, 0]);
    rot = mat4.create();
    mat4.rotate(rot, rot, Math.PI * window.kei, [0, 1, 0]);
    mat4.mul(mm, rot, mm);
    mat4.mul(mm, modelViewMat, mm);
    mat4.mul(mm, projectionMat, mm);
    gl.uniformMatrix4fv(program.uniform.modelViewMat, false, mm);

    gl.drawArrays(gl.TRIANGLES, 0, this.boardVert.length / 12);

    mm = mat4.create();
    mat4.translate(mm, mm, [20, -3, 0]);
    rot = mat4.create();
    mat4.rotate(rot, rot, Math.PI / 2, [0, 1, 0]);
    mat4.mul(mm, rot, mm);
    //mat4.mul(mm, modelViewMat, mm);
    mat4.mul(mm, projectionMat, mm);
    gl.uniformMatrix4fv(program.uniform.modelViewMat, false, mm);

    gl.drawArrays(gl.TRIANGLES, 0, this.boardVert.length / 12);

    if (timestamp) {
      mat4.fromRotation(this.heroRotationMat, timestamp / 2000, [0, 1, 0]);
      mat4.multiply(this.heroModelViewMat, modelViewMat, this.heroRotationMat);
      gl.uniformMatrix4fv(program.uniform.modelViewMat, false, this.heroModelViewMat);

      // We know that the additional model matrix is a pure rotation,
      // so we can just use the non-position parts of the matrix
      // directly, this is cheaper than the transpose+inverse that
      // normalFromMat4 would do.
      mat3.fromMat4(this.normalMat, this.heroRotationMat);
      gl.uniformMatrix3fv(program.uniform.normalMat, false, this.normalMat);

      //gl.drawElements(gl.TRIANGLES, this.heroCount, gl.UNSIGNED_SHORT, this.heroOffset * 2);
    }

    if (stats) {
      // To ensure that the FPS counter is visible in VR mode we have to
      // render it as part of the scene.
      mat4.fromTranslation(this.statsMat, [0, -0.3, -0.5]);
      mat4.scale(this.statsMat, this.statsMat, [0.3, 0.3, 0.3]);
      mat4.rotateX(this.statsMat, this.statsMat, -0.75);
      mat4.multiply(this.statsMat, modelViewMat, this.statsMat);
      //stats.render(projectionMat, this.statsMat);
    }
  };

  return CubeSea;
})();
